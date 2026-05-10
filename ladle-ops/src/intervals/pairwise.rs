use std::collections::HashMap;
use std::sync::Arc;

use arrow::array::{ArrayRef, Int64Array, Int64Builder, UInt32Array};
use arrow::error::ArrowError;
use arrow::record_batch::RecordBatch;
use coitrees::{COITree, Interval, IntervalTree};
use rayon::prelude::*;

use super::helpers::{nearest_schema, prefixed_schema, take_rows};
use super::schema::{get_interval, resolve_interval_cols};

pub fn overlap_batches(a: &RecordBatch, b: &RecordBatch) -> Result<RecordBatch, ArrowError> {
    let a_cols = resolve_interval_cols(a.schema_ref())
        .map_err(|e| ArrowError::InvalidArgumentError(e.to_string()))?;
    let b_cols = resolve_interval_cols(b.schema_ref())
        .map_err(|e| ArrowError::InvalidArgumentError(e.to_string()))?;

    let mut b_groups: HashMap<String, Vec<Interval<u32>>> = HashMap::new();
    for row in 0..b.num_rows() {
        if let Some((chrom, start, end)) = get_interval(b, &b_cols, row) {
            b_groups
                .entry(chrom.to_string())
                .or_default()
                .push(Interval::new(start, end - 1, row as u32));
        }
    }
    let trees: HashMap<String, COITree<u32, u32>> =
        b_groups.into_iter().map(|(k, v)| (k, COITree::new(&v))).collect();

    let mut a_groups: HashMap<String, Vec<u32>> = HashMap::new();
    for row in 0..a.num_rows() {
        if let Some((chrom, _, _)) = get_interval(a, &a_cols, row) {
            a_groups.entry(chrom.to_string()).or_default().push(row as u32);
        }
    }

    let pairs: Vec<(u32, u32)> = a_groups
        .par_iter()
        .flat_map(|(chrom, a_rows)| {
            let tree = match trees.get(chrom) {
                Some(t) => t,
                None => return vec![],
            };
            let mut local: Vec<(u32, u32)> = Vec::new();
            for &a_idx in a_rows {
                if let Some((_, start, end)) = get_interval(a, &a_cols, a_idx as usize) {
                    tree.query(start, end - 1, |hit| {
                        local.push((a_idx, *hit.metadata));
                    });
                }
            }
            local
        })
        .collect();

    if pairs.is_empty() {
        return Ok(RecordBatch::new_empty(prefixed_schema(a.schema_ref(), b.schema_ref())));
    }

    let a_indices = UInt32Array::from(pairs.iter().map(|&(i, _)| i).collect::<Vec<_>>());
    let b_indices = UInt32Array::from(pairs.iter().map(|&(_, j)| j).collect::<Vec<_>>());

    let schema = prefixed_schema(a.schema_ref(), b.schema_ref());
    let mut columns: Vec<ArrayRef> = take_rows(a, &a_indices)?;
    columns.extend(take_rows(b, &b_indices)?);
    RecordBatch::try_new(schema, columns)
}

pub fn nearest_batches(
    query: &RecordBatch,
    target: &RecordBatch,
) -> Result<RecordBatch, ArrowError> {
    let q_cols = resolve_interval_cols(query.schema_ref())
        .map_err(|e| ArrowError::InvalidArgumentError(e.to_string()))?;
    let t_cols = resolve_interval_cols(target.schema_ref())
        .map_err(|e| ArrowError::InvalidArgumentError(e.to_string()))?;

    let mut t_groups: HashMap<String, Vec<Interval<u32>>> = HashMap::new();
    let mut t_sorted: HashMap<String, Vec<(i32, i32, u32)>> = HashMap::new();
    for row in 0..target.num_rows() {
        if let Some((chrom, start, end)) = get_interval(target, &t_cols, row) {
            t_groups
                .entry(chrom.to_string())
                .or_default()
                .push(Interval::new(start, end - 1, row as u32));
            t_sorted.entry(chrom.to_string()).or_default().push((start, end, row as u32));
        }
    }
    let trees: HashMap<String, COITree<u32, u32>> =
        t_groups.into_iter().map(|(k, v)| (k, COITree::new(&v))).collect();
    let t_sorted: HashMap<String, Vec<(i32, i32, u32)>> = t_sorted
        .into_iter()
        .map(|(k, mut v)| {
            v.sort_unstable_by_key(|&(s, _, _)| s);
            (k, v)
        })
        .collect();

    let mut q_groups: HashMap<String, Vec<u32>> = HashMap::new();
    for row in 0..query.num_rows() {
        let chrom = match get_interval(query, &q_cols, row) {
            Some((ch, _, _)) => ch.to_string(),
            None => "\x00unmatched".to_string(),
        };
        q_groups.entry(chrom).or_default().push(row as u32);
    }

    let mut results: Vec<(u32, Option<u32>, Option<i64>)> = q_groups
        .par_iter()
        .flat_map(|(chrom, q_rows)| -> Vec<(u32, Option<u32>, Option<i64>)> {
            let tree = trees.get(chrom);
            let sorted = t_sorted.get(chrom);

            q_rows
                .iter()
                .map(|&q_idx| {
                    let (_, qs, qe) = match get_interval(query, &q_cols, q_idx as usize) {
                        Some(v) => v,
                        None => return (q_idx, None, None),
                    };
                    let (tree, sorted) = match (tree, sorted) {
                        (Some(t), Some(s)) => (t, s),
                        _ => return (q_idx, None, None),
                    };

                    let q_mid = (qs as i64 + qe as i64) / 2;
                    let mut best_overlap: Option<(i64, u32)> = None;
                    tree.query(qs, qe - 1, |hit| {
                        let ts = hit.first as i64;
                        let te = hit.last as i64 + 1;
                        let t_mid = (ts + te) / 2;
                        let diff = (q_mid - t_mid).abs();
                        let t_idx = *hit.metadata;
                        let better = match best_overlap {
                            None => true,
                            Some((d, bi)) => diff < d || (diff == d && t_idx < bi),
                        };
                        if better {
                            best_overlap = Some((diff, t_idx));
                        }
                    });
                    if let Some((_, t_idx)) = best_overlap {
                        return (q_idx, Some(t_idx), Some(0));
                    }

                    let pos = sorted.partition_point(|&(s, _, _)| s < qs);
                    let mut best: Option<(i64, u32)> = None;
                    let mut update = |dist: i64, t_idx: u32| {
                        if best.is_none_or(|(d, bi)| dist < d || (dist == d && t_idx < bi)) {
                            best = Some((dist, t_idx));
                        }
                    };
                    if pos > 0 {
                        let (_, te, t_idx) = sorted[pos - 1];
                        update((qs - te).max(0) as i64, t_idx);
                    }
                    if pos < sorted.len() {
                        let (ts, _, t_idx) = sorted[pos];
                        update((ts - qe).max(0) as i64, t_idx);
                    }

                    match best {
                        Some((dist, t_idx)) => (q_idx, Some(t_idx), Some(dist)),
                        None => (q_idx, None, None),
                    }
                })
                .collect()
        })
        .collect();

    results.sort_unstable_by_key(|&(q_idx, _, _)| q_idx);

    let schema = nearest_schema(query.schema_ref(), target.schema_ref());
    let n = query.num_rows();

    let q_indices = UInt32Array::from(results.iter().map(|&(i, _, _)| i).collect::<Vec<_>>());
    let mut columns: Vec<ArrayRef> = take_rows(query, &q_indices)?;

    let n_b_cols = target.num_columns();
    let matched_b: Vec<Option<u32>> = results.iter().map(|&(_, t, _)| t).collect();
    for col_idx in 0..n_b_cols {
        let col = target.column(col_idx);
        let idx_arr: Int64Array = matched_b.iter().map(|&t| t.map(|v| v as i64)).collect();
        let taken = arrow::compute::take(col.as_ref(), &idx_arr, None)?;
        columns.push(taken);
    }

    let mut dist_builder = Int64Builder::with_capacity(n);
    for &(_, _, d) in &results {
        match d {
            Some(v) => dist_builder.append_value(v),
            None => dist_builder.append_null(),
        }
    }
    columns.push(Arc::new(dist_builder.finish()) as ArrayRef);

    RecordBatch::try_new(schema, columns)
}

pub fn count_overlaps_batches(
    a: &RecordBatch,
    b: &RecordBatch,
) -> Result<RecordBatch, ArrowError> {
    let a_cols = resolve_interval_cols(a.schema_ref())
        .map_err(|e| ArrowError::InvalidArgumentError(e.to_string()))?;
    let b_cols = resolve_interval_cols(b.schema_ref())
        .map_err(|e| ArrowError::InvalidArgumentError(e.to_string()))?;

    let mut b_groups: HashMap<String, Vec<Interval<u32>>> = HashMap::new();
    for row in 0..b.num_rows() {
        if let Some((chrom, start, end)) = get_interval(b, &b_cols, row) {
            b_groups
                .entry(chrom.to_string())
                .or_default()
                .push(Interval::new(start, end - 1, row as u32));
        }
    }
    let trees: HashMap<String, COITree<u32, u32>> =
        b_groups.into_iter().map(|(k, v)| (k, COITree::new(&v))).collect();

    let mut a_groups: HashMap<String, Vec<u32>> = HashMap::new();
    for row in 0..a.num_rows() {
        if let Some((chrom, _, _)) = get_interval(a, &a_cols, row) {
            a_groups.entry(chrom.to_string()).or_default().push(row as u32);
        }
    }

    let mut counts: Vec<(u32, u32)> = a_groups
        .par_iter()
        .flat_map(|(chrom, a_rows)| {
            let tree = trees.get(chrom);
            a_rows
                .iter()
                .map(|&a_idx| {
                    let cnt = match tree {
                        Some(t) => match get_interval(a, &a_cols, a_idx as usize) {
                            Some((_, start, end)) => t.query_count(start, end - 1) as u32,
                            None => 0,
                        },
                        None => 0,
                    };
                    (a_idx, cnt)
                })
                .collect::<Vec<_>>()
        })
        .collect();

    counts.sort_unstable_by_key(|&(idx, _)| idx);

    let mut fields: Vec<arrow::datatypes::Field> =
        a.schema().fields().iter().map(|f| f.as_ref().clone()).collect();
    fields.push(arrow::datatypes::Field::new(
        "count",
        arrow::datatypes::DataType::UInt32,
        false,
    ));
    let schema = Arc::new(arrow::datatypes::Schema::new(fields));

    let mut columns: Vec<ArrayRef> = a.columns().to_vec();
    let count_array: arrow::array::UInt32Array = counts.iter().map(|&(_, c)| c).collect();
    columns.push(Arc::new(count_array) as ArrayRef);

    RecordBatch::try_new(schema, columns)
}
