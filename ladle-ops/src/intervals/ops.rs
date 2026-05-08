use std::collections::HashMap;
use std::sync::Arc;

use arrow::array::{ArrayRef, Int64Array, Int64Builder, RecordBatch, UInt32Array};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::error::ArrowError;
use coitrees::{COITree, GenericInterval, Interval, IntervalTree};
use pyo3::exceptions::PyIOError;
use pyo3::prelude::*;
use rayon::prelude::*;

use super::schema::{get_interval, resolve_interval_cols};
use crate::arrow_utils::{batch_to_pyarrow, pyarrow_to_batch};

// ---------------------------------------------------------------------------
// Output schema helpers
// ---------------------------------------------------------------------------

fn prefixed_schema(a: &Schema, b: &Schema) -> Arc<Schema> {
    let fields: Vec<Field> = a
        .fields()
        .iter()
        .map(|f| {
            Field::new(
                format!("a_{}", f.name()),
                f.data_type().clone(),
                f.is_nullable(),
            )
        })
        .chain(b.fields().iter().map(|f| {
            Field::new(
                format!("b_{}", f.name()),
                f.data_type().clone(),
                f.is_nullable(),
            )
        }))
        .collect();
    Arc::new(Schema::new(fields))
}

fn nearest_schema(a: &Schema, b: &Schema) -> Arc<Schema> {
    let mut fields: Vec<Field> = prefixed_schema(a, b)
        .fields()
        .iter()
        .map(|f| f.as_ref().clone())
        .collect();
    fields.push(Field::new("distance", DataType::Int64, true));
    Arc::new(Schema::new(fields))
}

// ---------------------------------------------------------------------------
// Arrow take helper: select rows by index array from a RecordBatch
// ---------------------------------------------------------------------------

fn take_rows(batch: &RecordBatch, indices: &UInt32Array) -> Result<Vec<ArrayRef>, ArrowError> {
    let idx = arrow::array::cast::as_primitive_array::<arrow::datatypes::UInt32Type>(indices);
    batch
        .columns()
        .iter()
        .map(|col| arrow::compute::take(col.as_ref(), idx, None))
        .collect()
}

// ---------------------------------------------------------------------------
// overlap
// ---------------------------------------------------------------------------

fn overlap_batches(a: &RecordBatch, b: &RecordBatch) -> Result<RecordBatch, ArrowError> {
    let a_cols = resolve_interval_cols(a.schema_ref())
        .map_err(|e| ArrowError::InvalidArgumentError(e.to_string()))?;
    let b_cols = resolve_interval_cols(b.schema_ref())
        .map_err(|e| ArrowError::InvalidArgumentError(e.to_string()))?;

    // Build COITree per chrom from b
    let mut b_groups: HashMap<String, Vec<Interval<u32>>> = HashMap::new();
    for row in 0..b.num_rows() {
        if let Some((chrom, start, end)) = get_interval(b, &b_cols, row) {
            b_groups
                .entry(chrom.to_string())
                .or_default()
                .push(Interval::new(start, end - 1, row as u32)); // coitrees: closed [start, end]
        }
    }
    let trees: HashMap<String, COITree<u32, u32>> = b_groups
        .into_iter()
        .map(|(k, v)| (k, COITree::new(&v)))
        .collect();

    // Group a rows by chrom
    let mut a_groups: HashMap<String, Vec<u32>> = HashMap::new();
    for row in 0..a.num_rows() {
        if let Some((chrom, _, _)) = get_interval(a, &a_cols, row) {
            a_groups
                .entry(chrom.to_string())
                .or_default()
                .push(row as u32);
        }
    }

    // Rayon: query each chrom group in parallel
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
                        local.push((a_idx, *hit.metadata()));
                    });
                }
            }
            local
        })
        .collect();

    if pairs.is_empty() {
        return Ok(RecordBatch::new_empty(prefixed_schema(
            a.schema_ref(),
            b.schema_ref(),
        )));
    }

    let a_indices = UInt32Array::from(pairs.iter().map(|&(i, _)| i).collect::<Vec<_>>());
    let b_indices = UInt32Array::from(pairs.iter().map(|&(_, j)| j).collect::<Vec<_>>());

    let schema = prefixed_schema(a.schema_ref(), b.schema_ref());
    let mut columns: Vec<ArrayRef> = take_rows(a, &a_indices)?;
    columns.extend(take_rows(b, &b_indices)?);
    RecordBatch::try_new(schema, columns)
}

// ---------------------------------------------------------------------------
// nearest
// ---------------------------------------------------------------------------

fn nearest_batches(query: &RecordBatch, target: &RecordBatch) -> Result<RecordBatch, ArrowError> {
    let q_cols = resolve_interval_cols(query.schema_ref())
        .map_err(|e| ArrowError::InvalidArgumentError(e.to_string()))?;
    let t_cols = resolve_interval_cols(target.schema_ref())
        .map_err(|e| ArrowError::InvalidArgumentError(e.to_string()))?;

    // Build per-chrom COITree + sorted Vec from target
    let mut t_groups: HashMap<String, Vec<Interval<u32>>> = HashMap::new();
    let mut t_sorted: HashMap<String, Vec<(i32, i32, u32)>> = HashMap::new();
    for row in 0..target.num_rows() {
        if let Some((chrom, start, end)) = get_interval(target, &t_cols, row) {
            t_groups
                .entry(chrom.to_string())
                .or_default()
                .push(Interval::new(start, end - 1, row as u32));
            t_sorted
                .entry(chrom.to_string())
                .or_default()
                .push((start, end, row as u32));
        }
    }
    let trees: HashMap<String, COITree<u32, u32>> = t_groups
        .into_iter()
        .map(|(k, v)| (k, COITree::new(&v)))
        .collect();
    // Sort each chrom's vec by start for binary search
    let t_sorted: HashMap<String, Vec<(i32, i32, u32)>> = t_sorted
        .into_iter()
        .map(|(k, mut v)| {
            v.sort_unstable_by_key(|&(s, _, _)| s);
            (k, v)
        })
        .collect();

    // Group query rows by chrom
    let mut q_groups: HashMap<String, Vec<u32>> = HashMap::new();
    for row in 0..query.num_rows() {
        let chrom = match get_interval(query, &q_cols, row) {
            Some((ch, _, _)) => ch.to_string(),
            None => "\x00unmatched".to_string(),
        };
        q_groups.entry(chrom).or_default().push(row as u32);
    }

    // Rayon: process each chrom group in parallel
    // Returns Vec<(query_idx, Option<target_idx>, Option<distance>)>
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

                    // Check for overlap first (distance = 0)
                    let q_mid = (qs as i64 + qe as i64) / 2;
                    let mut best_overlap: Option<(i64, u32)> = None; // (|q_mid - t_mid|, t_idx)
                    tree.query(qs, qe - 1, |hit| {
                        let ts = hit.first as i64;
                        let te = hit.last as i64 + 1;
                        let t_mid = (ts + te) / 2;
                        let diff = (q_mid - t_mid).abs();
                        let t_idx = *hit.metadata();
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

                    // Binary search for nearest non-overlapping
                    // Find closest by min distance to endpoints
                    let pos = sorted.partition_point(|&(s, _, _)| s < qs);

                    let mut best: Option<(i64, u32)> = None; // (distance, t_idx)
                    let mut update = |dist: i64, t_idx: u32| {
                        if best.is_none_or(|(d, bi)| dist < d || (dist == d && t_idx < bi)) {
                            best = Some((dist, t_idx));
                        }
                    };

                    // Check intervals ending before query (upstream)
                    if pos > 0 {
                        let (_, te, t_idx) = sorted[pos - 1];
                        update((qs - te).max(0) as i64, t_idx);
                    }
                    // Check interval starting at or after query start (downstream)
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

    // Restore original query order
    results.sort_unstable_by_key(|&(q_idx, _, _)| q_idx);

    // Build output batch
    let schema = nearest_schema(query.schema_ref(), target.schema_ref());
    let n = query.num_rows();

    let q_indices = UInt32Array::from(results.iter().map(|&(i, _, _)| i).collect::<Vec<_>>());
    let mut columns: Vec<ArrayRef> = take_rows(query, &q_indices)?;

    // b_* columns — nullable, use null for unmatched rows
    let n_b_cols = target.num_columns();
    let matched_b: Vec<Option<u32>> = results.iter().map(|&(_, t, _)| t).collect();

    for col_idx in 0..n_b_cols {
        let col = target.column(col_idx);
        // Build indices array with nulls for unmatched rows
        let idx_arr: Int64Array = matched_b.iter().map(|&t| t.map(|v| v as i64)).collect();
        let taken = arrow::compute::take(col.as_ref(), &idx_arr, None)?;
        columns.push(taken);
    }

    // distance column
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

// ---------------------------------------------------------------------------
// count_overlaps
// ---------------------------------------------------------------------------

fn count_overlaps_batches(a: &RecordBatch, b: &RecordBatch) -> Result<RecordBatch, ArrowError> {
    let a_cols = resolve_interval_cols(a.schema_ref())
        .map_err(|e| ArrowError::InvalidArgumentError(e.to_string()))?;
    let b_cols = resolve_interval_cols(b.schema_ref())
        .map_err(|e| ArrowError::InvalidArgumentError(e.to_string()))?;

    // Build COITree per chrom from b
    let mut b_groups: HashMap<String, Vec<Interval<u32>>> = HashMap::new();
    for row in 0..b.num_rows() {
        if let Some((chrom, start, end)) = get_interval(b, &b_cols, row) {
            b_groups
                .entry(chrom.to_string())
                .or_default()
                .push(Interval::new(start, end - 1, row as u32));
        }
    }
    let trees: HashMap<String, COITree<u32, u32>> = b_groups
        .into_iter()
        .map(|(k, v)| (k, COITree::new(&v)))
        .collect();

    // Count overlaps for each a row (parallel by chrom)
    let mut a_groups: HashMap<String, Vec<u32>> = HashMap::new();
    for row in 0..a.num_rows() {
        if let Some((chrom, _, _)) = get_interval(a, &a_cols, row) {
            a_groups
                .entry(chrom.to_string())
                .or_default()
                .push(row as u32);
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

    // Build output: all a columns + count column
    let mut fields: Vec<arrow::datatypes::Field> = a
        .schema()
        .fields()
        .iter()
        .map(|f| f.as_ref().clone())
        .collect();
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

// ---------------------------------------------------------------------------
// Shared helpers for single-batch sweep operations
// ---------------------------------------------------------------------------

/// Collect (chrom, start, end, original_row_idx) for all valid rows, sorted by (chrom, start).
fn sorted_intervals(batch: &RecordBatch) -> Result<Vec<(String, i32, i32, usize)>, ArrowError> {
    let cols = resolve_interval_cols(batch.schema_ref())
        .map_err(|e| ArrowError::InvalidArgumentError(e.to_string()))?;
    let mut rows: Vec<(String, i32, i32, usize)> = (0..batch.num_rows())
        .filter_map(|i| {
            let (ch, s, e) = get_interval(batch, &cols, i)?;
            Some((ch.to_string(), s, e, i))
        })
        .collect();
    rows.sort_unstable_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
    Ok(rows)
}

/// Build a simple chrom/start/end RecordBatch from a list of (chrom, start, end) tuples.
fn chrom_start_end_batch(rows: Vec<(String, i32, i32)>) -> Result<RecordBatch, ArrowError> {
    use arrow::array::{Int32Array, StringArray};
    let schema = Arc::new(Schema::new(vec![
        Field::new("chrom", DataType::Utf8, false),
        Field::new("start", DataType::Int32, false),
        Field::new("end", DataType::Int32, false),
    ]));
    let chroms: StringArray = rows.iter().map(|(c, _, _)| Some(c.as_str())).collect();
    let starts: Int32Array = rows.iter().map(|(_, s, _)| *s).collect();
    let ends: Int32Array = rows.iter().map(|(_, _, e)| *e).collect();
    RecordBatch::try_new(
        schema,
        vec![
            Arc::new(chroms) as ArrayRef,
            Arc::new(starts) as ArrayRef,
            Arc::new(ends) as ArrayRef,
        ],
    )
}

// ---------------------------------------------------------------------------
// cluster
// ---------------------------------------------------------------------------

fn cluster_batches(batch: &RecordBatch) -> Result<RecordBatch, ArrowError> {
    let sorted = sorted_intervals(batch)?;

    // Assign cluster_id per sorted row, then restore original order
    let mut assignments: Vec<(usize, u32)> = Vec::with_capacity(sorted.len()); // (orig_idx, cluster)
    let mut cluster_id: u32 = 0;
    let mut cur_chrom = String::new();
    let mut cur_end: i32 = i32::MIN;

    for (chrom, start, end, orig_idx) in &sorted {
        if *chrom != cur_chrom {
            if !cur_chrom.is_empty() {
                cluster_id += 1;
            }
            cur_chrom = chrom.clone();
            cur_end = *end;
        } else if *start >= cur_end {
            cluster_id += 1;
            cur_end = *end;
        } else {
            cur_end = cur_end.max(*end);
        }
        assignments.push((*orig_idx, cluster_id));
    }

    // Restore original row order
    assignments.sort_unstable_by_key(|&(orig, _)| orig);

    let mut fields: Vec<Field> = batch
        .schema()
        .fields()
        .iter()
        .map(|f| f.as_ref().clone())
        .collect();
    fields.push(Field::new("cluster_id", DataType::UInt32, false));
    let schema = Arc::new(Schema::new(fields));

    let mut columns = batch.columns().to_vec();
    let cluster_arr: UInt32Array = assignments.iter().map(|&(_, c)| c).collect();
    columns.push(Arc::new(cluster_arr) as ArrayRef);
    RecordBatch::try_new(schema, columns)
}

// ---------------------------------------------------------------------------
// merge
// ---------------------------------------------------------------------------

fn merge_batches(batch: &RecordBatch) -> Result<RecordBatch, ArrowError> {
    let sorted = sorted_intervals(batch)?;
    let mut merged: Vec<(String, i32, i32)> = Vec::new();

    for (chrom, start, end, _) in sorted {
        match merged.last_mut() {
            Some(last) if last.0 == chrom && start < last.2 => {
                last.2 = last.2.max(end);
            }
            _ => merged.push((chrom, start, end)),
        }
    }
    chrom_start_end_batch(merged)
}

// ---------------------------------------------------------------------------
// subtract
// ---------------------------------------------------------------------------

fn subtract_batches(a: &RecordBatch, b: &RecordBatch) -> Result<RecordBatch, ArrowError> {
    let a_cols = resolve_interval_cols(a.schema_ref())
        .map_err(|e| ArrowError::InvalidArgumentError(e.to_string()))?;
    let b_cols = resolve_interval_cols(b.schema_ref())
        .map_err(|e| ArrowError::InvalidArgumentError(e.to_string()))?;

    // Build COITree per chrom from b
    let mut b_groups: HashMap<String, Vec<Interval<u32>>> = HashMap::new();
    for row in 0..b.num_rows() {
        if let Some((chrom, start, end)) = get_interval(b, &b_cols, row) {
            b_groups
                .entry(chrom.to_string())
                .or_default()
                .push(Interval::new(start, end - 1, row as u32));
        }
    }
    let trees: HashMap<String, COITree<u32, u32>> = b_groups
        .into_iter()
        .map(|(k, v)| (k, COITree::new(&v)))
        .collect();

    // Keep a-rows with zero overlap
    let keep: Vec<u32> = (0..a.num_rows() as u32)
        .filter(|&row| match get_interval(a, &a_cols, row as usize) {
            None => false,
            Some((chrom, start, end)) => match trees.get(chrom) {
                None => true,
                Some(t) => t.query_count(start, end - 1) == 0,
            },
        })
        .collect();

    if keep.is_empty() {
        return Ok(RecordBatch::new_empty(a.schema()));
    }
    let indices = UInt32Array::from(keep);
    let columns: Result<Vec<ArrayRef>, _> = take_rows(a, &indices);
    RecordBatch::try_new(a.schema(), columns?)
}

// ---------------------------------------------------------------------------
// complement
// ---------------------------------------------------------------------------

fn complement_batches(
    batch: &RecordBatch,
    chrom_sizes: Option<&HashMap<String, i32>>,
) -> Result<RecordBatch, ArrowError> {
    let sorted = sorted_intervals(batch)?;
    let mut gaps: Vec<(String, i32, i32)> = Vec::new();

    let mut cur_chrom = String::new();
    let mut cur_end: i32 = 0;

    for (chrom, start, end, _) in &sorted {
        if *chrom != cur_chrom {
            // Emit trailing gap for previous chrom
            if let Some(sizes) = chrom_sizes
                && let Some(&sz) = sizes.get(&cur_chrom)
                && cur_end < sz
            {
                gaps.push((cur_chrom.clone(), cur_end, sz));
            }
            cur_chrom = chrom.clone();
            cur_end = 0;
        }
        if *start > cur_end {
            gaps.push((chrom.clone(), cur_end, *start));
        }
        cur_end = cur_end.max(*end);
    }
    // Trailing gap for last chrom
    if let Some(sizes) = chrom_sizes
        && let Some(&sz) = sizes.get(&cur_chrom)
        && cur_end < sz
    {
        gaps.push((cur_chrom, cur_end, sz));
    }

    chrom_start_end_batch(gaps)
}

// ---------------------------------------------------------------------------
// coverage
// ---------------------------------------------------------------------------

fn coverage_batches(batch: &RecordBatch) -> Result<RecordBatch, ArrowError> {
    let cols = resolve_interval_cols(batch.schema_ref())
        .map_err(|e| ArrowError::InvalidArgumentError(e.to_string()))?;

    // Group events by chrom
    let mut chrom_events: HashMap<String, Vec<(i32, i32)>> = HashMap::new(); // (pos, +1/-1)
    for row in 0..batch.num_rows() {
        if let Some((chrom, start, end)) = get_interval(batch, &cols, row) {
            let ev = chrom_events.entry(chrom.to_string()).or_default();
            ev.push((start, 1));
            ev.push((end, -1));
        }
    }

    let mut result: Vec<(String, i32, i32, u32)> = Vec::new();

    for (chrom, mut events) in chrom_events {
        events.sort_unstable_by_key(|&(pos, _)| pos);
        let mut depth: i32 = 0;
        let mut pos = events[0].0;
        for (ep, delta) in &events {
            if *ep > pos && depth > 0 {
                result.push((chrom.clone(), pos, *ep, depth as u32));
            }
            if *ep != pos {
                pos = *ep;
            }
            depth += delta;
        }
    }
    result.sort_unstable_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));

    use arrow::array::{Int32Array, StringArray, UInt32Array as UA};
    let schema = Arc::new(Schema::new(vec![
        Field::new("chrom", DataType::Utf8, false),
        Field::new("start", DataType::Int32, false),
        Field::new("end", DataType::Int32, false),
        Field::new("depth", DataType::UInt32, false),
    ]));
    let chroms: StringArray = result.iter().map(|(c, _, _, _)| Some(c.as_str())).collect();
    let starts: Int32Array = result.iter().map(|(_, s, _, _)| *s).collect();
    let ends: Int32Array = result.iter().map(|(_, _, e, _)| *e).collect();
    let depths: UA = result.iter().map(|(_, _, _, d)| *d).collect();
    RecordBatch::try_new(
        schema,
        vec![
            Arc::new(chroms) as ArrayRef,
            Arc::new(starts) as ArrayRef,
            Arc::new(ends) as ArrayRef,
            Arc::new(depths) as ArrayRef,
        ],
    )
}

// ---------------------------------------------------------------------------
// expand
// ---------------------------------------------------------------------------

fn expand_batches(
    batch: &RecordBatch,
    amount: i32,
    start_amount: Option<i32>,
    end_amount: Option<i32>,
) -> Result<RecordBatch, ArrowError> {
    use arrow::array::Int32Array;

    let cols = resolve_interval_cols(batch.schema_ref())
        .map_err(|e| ArrowError::InvalidArgumentError(e.to_string()))?;

    let l = start_amount.unwrap_or(amount);
    let r = end_amount.unwrap_or(amount);

    let old_start = batch
        .column(cols.start)
        .as_any()
        .downcast_ref::<Int32Array>()
        .ok_or_else(|| ArrowError::InvalidArgumentError("start column is not Int32".into()))?;
    let old_end = batch
        .column(cols.end)
        .as_any()
        .downcast_ref::<Int32Array>()
        .ok_or_else(|| ArrowError::InvalidArgumentError("end column is not Int32".into()))?;

    let new_start: Int32Array = (0..batch.num_rows())
        .map(|i| (old_start.value(i) - l).max(1))
        .collect();
    let new_end: Int32Array = (0..batch.num_rows())
        .map(|i| old_end.value(i) + r)
        .collect();

    let mut columns = batch.columns().to_vec();
    columns[cols.start] = Arc::new(new_start) as ArrayRef;
    columns[cols.end] = Arc::new(new_end) as ArrayRef;
    RecordBatch::try_new(batch.schema(), columns)
}

// ---------------------------------------------------------------------------
// shift
// ---------------------------------------------------------------------------

fn shift_batches(batch: &RecordBatch, amount: i32) -> Result<RecordBatch, ArrowError> {
    use arrow::array::Int32Array;

    let cols = resolve_interval_cols(batch.schema_ref())
        .map_err(|e| ArrowError::InvalidArgumentError(e.to_string()))?;

    let old_start = batch
        .column(cols.start)
        .as_any()
        .downcast_ref::<Int32Array>()
        .ok_or_else(|| ArrowError::InvalidArgumentError("start column is not Int32".into()))?;
    let old_end = batch
        .column(cols.end)
        .as_any()
        .downcast_ref::<Int32Array>()
        .ok_or_else(|| ArrowError::InvalidArgumentError("end column is not Int32".into()))?;

    let new_start: Int32Array = (0..batch.num_rows())
        .map(|i| (old_start.value(i) + amount).max(1))
        .collect();
    let new_end: Int32Array = (0..batch.num_rows())
        .map(|i| {
            let s = old_start.value(i) + amount;
            if s < 1 {
                // clamp start to 1 but preserve width
                old_end.value(i) - old_start.value(i) + 1
            } else {
                old_end.value(i) + amount
            }
        })
        .collect();

    let mut columns = batch.columns().to_vec();
    columns[cols.start] = Arc::new(new_start) as ArrayRef;
    columns[cols.end] = Arc::new(new_end) as ArrayRef;
    RecordBatch::try_new(batch.schema(), columns)
}

// ---------------------------------------------------------------------------
// sort_bedframe
// ---------------------------------------------------------------------------

fn sort_bedframe_batches(batch: &RecordBatch, natural: bool) -> Result<RecordBatch, ArrowError> {
    use arrow::array::{Int32Array, StringArray};
    use arrow::compute::{SortColumn, SortOptions, lexsort_to_indices};

    let cols = resolve_interval_cols(batch.schema_ref())
        .map_err(|e| ArrowError::InvalidArgumentError(e.to_string()))?;

    let opts = SortOptions {
        descending: false,
        nulls_first: false,
    };

    let sort_indices = if natural {
        // Build a synthetic chrom-key column for natural order:
        // extract leading alpha prefix + numeric suffix
        let chrom_col = batch
            .column(cols.chrom)
            .as_any()
            .downcast_ref::<StringArray>()
            .ok_or_else(|| ArrowError::InvalidArgumentError("chrom not Utf8".into()))?;

        // Represent each chrom as (prefix: String, num: i64) packed as two arrays
        let mut prefixes: Vec<String> = Vec::with_capacity(batch.num_rows());
        let mut nums: Vec<i64> = Vec::with_capacity(batch.num_rows());
        for i in 0..batch.num_rows() {
            let s = chrom_col.value(i);
            let split_pos = s.len() - s.chars().rev().take_while(|c| c.is_ascii_digit()).count();
            prefixes.push(s[..split_pos].to_string());
            nums.push(s[split_pos..].parse::<i64>().unwrap_or(i64::MAX));
        }
        let prefix_arr: StringArray = prefixes.iter().map(|s| Some(s.as_str())).collect();
        let num_arr: arrow::array::Int64Array = nums.into_iter().collect();
        let start_col = batch
            .column(cols.start)
            .as_any()
            .downcast_ref::<Int32Array>()
            .ok_or_else(|| ArrowError::InvalidArgumentError("start not Int32".into()))?;

        lexsort_to_indices(
            &[
                SortColumn {
                    values: Arc::new(prefix_arr) as ArrayRef,
                    options: Some(opts),
                },
                SortColumn {
                    values: Arc::new(num_arr) as ArrayRef,
                    options: Some(opts),
                },
                SortColumn {
                    values: Arc::new(start_col.clone()) as ArrayRef,
                    options: Some(opts),
                },
            ],
            None,
        )?
    } else {
        lexsort_to_indices(
            &[
                SortColumn {
                    values: batch.column(cols.chrom).clone(),
                    options: Some(opts),
                },
                SortColumn {
                    values: batch.column(cols.start).clone(),
                    options: Some(opts),
                },
            ],
            None,
        )?
    };

    let columns: Result<Vec<ArrayRef>, _> = batch
        .columns()
        .iter()
        .map(|col| arrow::compute::take(col.as_ref(), &sort_indices, None))
        .collect();
    RecordBatch::try_new(batch.schema(), columns?)
}

// ---------------------------------------------------------------------------
// flank
// ---------------------------------------------------------------------------

fn flank_batches(batch: &RecordBatch, width: i32, start: bool) -> Result<RecordBatch, ArrowError> {
    use arrow::array::Int32Array;

    let cols = resolve_interval_cols(batch.schema_ref())
        .map_err(|e| ArrowError::InvalidArgumentError(e.to_string()))?;

    let old_start = batch
        .column(cols.start)
        .as_any()
        .downcast_ref::<Int32Array>()
        .ok_or_else(|| ArrowError::InvalidArgumentError("start column is not Int32".into()))?;
    let old_end = batch
        .column(cols.end)
        .as_any()
        .downcast_ref::<Int32Array>()
        .ok_or_else(|| ArrowError::InvalidArgumentError("end column is not Int32".into()))?;

    let (new_start, new_end): (Int32Array, Int32Array) = if start {
        // flank placed before: [start - width, start)
        let ns = (0..batch.num_rows())
            .map(|i| (old_start.value(i) - width).max(1))
            .collect();
        let ne = (0..batch.num_rows()).map(|i| old_start.value(i)).collect();
        (ns, ne)
    } else {
        // flank placed after: [end, end + width)
        let ns = (0..batch.num_rows()).map(|i| old_end.value(i)).collect();
        let ne = (0..batch.num_rows())
            .map(|i| old_end.value(i) + width)
            .collect();
        (ns, ne)
    };

    let mut columns = batch.columns().to_vec();
    columns[cols.start] = Arc::new(new_start) as ArrayRef;
    columns[cols.end] = Arc::new(new_end) as ArrayRef;
    RecordBatch::try_new(batch.schema(), columns)
}

// ---------------------------------------------------------------------------
// set_width
// ---------------------------------------------------------------------------

fn set_width_batches(
    batch: &RecordBatch,
    width: i32,
    anchor: &str,
) -> Result<RecordBatch, ArrowError> {
    use arrow::array::Int32Array;

    let cols = resolve_interval_cols(batch.schema_ref())
        .map_err(|e| ArrowError::InvalidArgumentError(e.to_string()))?;

    let old_start = batch
        .column(cols.start)
        .as_any()
        .downcast_ref::<Int32Array>()
        .ok_or_else(|| ArrowError::InvalidArgumentError("start column is not Int32".into()))?;
    let old_end = batch
        .column(cols.end)
        .as_any()
        .downcast_ref::<Int32Array>()
        .ok_or_else(|| ArrowError::InvalidArgumentError("end column is not Int32".into()))?;

    let (new_start, new_end): (Int32Array, Int32Array) = match anchor {
        "start" => {
            let ns = (0..batch.num_rows()).map(|i| old_start.value(i)).collect();
            let ne = (0..batch.num_rows())
                .map(|i| old_start.value(i) + width)
                .collect();
            (ns, ne)
        }
        "end" => {
            let ns = (0..batch.num_rows())
                .map(|i| (old_end.value(i) - width).max(1))
                .collect();
            let ne = (0..batch.num_rows()).map(|i| old_end.value(i)).collect();
            (ns, ne)
        }
        "center" => {
            let ns = (0..batch.num_rows())
                .map(|i| {
                    let mid = (old_start.value(i) + old_end.value(i)) / 2;
                    (mid - width / 2).max(1)
                })
                .collect();
            let ne = (0..batch.num_rows())
                .map(|i| {
                    let mid = (old_start.value(i) + old_end.value(i)) / 2;
                    mid + (width + 1) / 2
                })
                .collect();
            (ns, ne)
        }
        other => {
            return Err(ArrowError::InvalidArgumentError(format!(
                "anchor must be 'start', 'end', or 'center'; got '{}'",
                other
            )));
        }
    };

    let mut columns = batch.columns().to_vec();
    columns[cols.start] = Arc::new(new_start) as ArrayRef;
    columns[cols.end] = Arc::new(new_end) as ArrayRef;
    RecordBatch::try_new(batch.schema(), columns)
}

// ---------------------------------------------------------------------------
// tile
// ---------------------------------------------------------------------------

fn tile_batches(batch: &RecordBatch, width: i32) -> Result<RecordBatch, ArrowError> {
    use arrow::array::Int32Array;

    if width <= 0 {
        return Err(ArrowError::InvalidArgumentError("width must be > 0".into()));
    }

    let cols = resolve_interval_cols(batch.schema_ref())
        .map_err(|e| ArrowError::InvalidArgumentError(e.to_string()))?;

    let old_start = batch
        .column(cols.start)
        .as_any()
        .downcast_ref::<Int32Array>()
        .ok_or_else(|| ArrowError::InvalidArgumentError("start column is not Int32".into()))?;
    let old_end = batch
        .column(cols.end)
        .as_any()
        .downcast_ref::<Int32Array>()
        .ok_or_else(|| ArrowError::InvalidArgumentError("end column is not Int32".into()))?;

    // Build row-repeat indices and new start/end values
    let mut row_indices: Vec<u32> = Vec::new();
    let mut tile_starts: Vec<i32> = Vec::new();
    let mut tile_ends: Vec<i32> = Vec::new();

    for row in 0..batch.num_rows() {
        let s = old_start.value(row);
        let e = old_end.value(row);
        let mut cur = s;
        while cur < e {
            let next = (cur + width).min(e);
            row_indices.push(row as u32);
            tile_starts.push(cur);
            tile_ends.push(next);
            cur = next;
        }
    }

    if row_indices.is_empty() {
        return Ok(RecordBatch::new_empty(batch.schema()));
    }

    let indices = UInt32Array::from(row_indices);
    let mut columns = take_rows(batch, &indices)?;
    // Replace start/end with tile boundaries
    let new_start: Int32Array = tile_starts.into_iter().collect();
    let new_end: Int32Array = tile_ends.into_iter().collect();
    columns[cols.start] = Arc::new(new_start) as ArrayRef;
    columns[cols.end] = Arc::new(new_end) as ArrayRef;
    RecordBatch::try_new(batch.schema(), columns)
}

// ---------------------------------------------------------------------------
// disjoin
// ---------------------------------------------------------------------------

fn disjoin_batches(batch: &RecordBatch) -> Result<RecordBatch, ArrowError> {
    let sorted = sorted_intervals(batch)?;
    let mut result: Vec<(String, i32, i32)> = Vec::new();

    // Group by chrom, collect all boundary points, emit covered sub-intervals
    let mut i = 0;
    while i < sorted.len() {
        let chrom = sorted[i].0.clone();
        let j = sorted[i..]
            .iter()
            .position(|(c, _, _, _)| c != &chrom)
            .map(|p| i + p)
            .unwrap_or(sorted.len());

        // Collect all boundary points for this chrom
        let mut pts: Vec<i32> = Vec::new();
        for row in &sorted[i..j] {
            pts.push(row.1);
            pts.push(row.2);
        }
        pts.sort_unstable();
        pts.dedup();

        // For each consecutive pair, check if any interval covers it
        for w in pts.windows(2) {
            let (bp_s, bp_e) = (w[0], w[1]);
            let covered = sorted[i..j]
                .iter()
                .any(|(_, s, e, _)| *s <= bp_s && *e >= bp_e);
            if covered {
                result.push((chrom.clone(), bp_s, bp_e));
            }
        }
        i = j;
    }
    chrom_start_end_batch(result)
}

// ---------------------------------------------------------------------------
// intersect_ranges / union_ranges / setdiff_ranges
// ---------------------------------------------------------------------------

/// Regions covered by both A and B: clip overlapping pairs to their intersection, then merge.
fn intersect_ranges_batches(a: &RecordBatch, b: &RecordBatch) -> Result<RecordBatch, ArrowError> {
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
    let trees: HashMap<String, COITree<u32, u32>> = b_groups
        .into_iter()
        .map(|(k, v)| (k, COITree::new(&v)))
        .collect();

    let mut raw: Vec<(String, i32, i32)> = Vec::new();
    for row in 0..a.num_rows() {
        if let Some((chrom, as_, ae)) = get_interval(a, &a_cols, row)
            && let Some(tree) = trees.get(chrom)
        {
            tree.query(as_, ae - 1, |hit| {
                let bs = hit.first;
                let be = hit.last + 1;
                raw.push((chrom.to_string(), as_.max(bs), ae.min(be)));
            });
        }
    }
    // Merge the clipped pieces
    raw.sort_unstable_by(|x, y| x.0.cmp(&y.0).then(x.1.cmp(&y.1)));
    let mut merged: Vec<(String, i32, i32)> = Vec::new();
    for (chrom, start, end) in raw {
        match merged.last_mut() {
            Some(last) if last.0 == chrom && start < last.2 => {
                last.2 = last.2.max(end);
            }
            _ => merged.push((chrom, start, end)),
        }
    }
    chrom_start_end_batch(merged)
}

/// Regions covered by A or B (union = merge of combined input).
fn union_ranges_batches(a: &RecordBatch, b: &RecordBatch) -> Result<RecordBatch, ArrowError> {
    let a_cols = resolve_interval_cols(a.schema_ref())
        .map_err(|e| ArrowError::InvalidArgumentError(e.to_string()))?;
    let b_cols = resolve_interval_cols(b.schema_ref())
        .map_err(|e| ArrowError::InvalidArgumentError(e.to_string()))?;

    let mut combined: Vec<(String, i32, i32)> = Vec::new();
    for row in 0..a.num_rows() {
        if let Some((ch, s, e)) = get_interval(a, &a_cols, row) {
            combined.push((ch.to_string(), s, e));
        }
    }
    for row in 0..b.num_rows() {
        if let Some((ch, s, e)) = get_interval(b, &b_cols, row) {
            combined.push((ch.to_string(), s, e));
        }
    }
    combined.sort_unstable_by(|x, y| x.0.cmp(&y.0).then(x.1.cmp(&y.1)));

    let mut merged: Vec<(String, i32, i32)> = Vec::new();
    for (chrom, start, end) in combined {
        match merged.last_mut() {
            Some(last) if last.0 == chrom && start < last.2 => {
                last.2 = last.2.max(end);
            }
            _ => merged.push((chrom, start, end)),
        }
    }
    chrom_start_end_batch(merged)
}

/// Regions in A not covered by B: clip A intervals around B overlaps.
fn setdiff_ranges_batches(a: &RecordBatch, b: &RecordBatch) -> Result<RecordBatch, ArrowError> {
    let a_cols = resolve_interval_cols(a.schema_ref())
        .map_err(|e| ArrowError::InvalidArgumentError(e.to_string()))?;
    let b_cols = resolve_interval_cols(b.schema_ref())
        .map_err(|e| ArrowError::InvalidArgumentError(e.to_string()))?;

    // Build sorted B per chrom for clipping
    let mut b_sorted: HashMap<String, Vec<(i32, i32)>> = HashMap::new();
    for row in 0..b.num_rows() {
        if let Some((chrom, start, end)) = get_interval(b, &b_cols, row) {
            b_sorted
                .entry(chrom.to_string())
                .or_default()
                .push((start, end));
        }
    }
    // Merge B per chrom so clipping is clean
    for v in b_sorted.values_mut() {
        v.sort_unstable();
        let mut merged: Vec<(i32, i32)> = Vec::new();
        for &(s, e) in v.iter() {
            match merged.last_mut() {
                Some(last) if s < last.1 => {
                    last.1 = last.1.max(e);
                }
                _ => merged.push((s, e)),
            }
        }
        *v = merged;
    }

    let mut result: Vec<(String, i32, i32)> = Vec::new();
    for row in 0..a.num_rows() {
        if let Some((chrom, mut cur_s, a_e)) = get_interval(a, &a_cols, row) {
            let chrom_s = chrom.to_string();
            match b_sorted.get(chrom) {
                None => result.push((chrom_s, cur_s, a_e)),
                Some(b_ivs) => {
                    // Walk B intervals that overlap [cur_s, a_e)
                    let start_pos = b_ivs.partition_point(|&(_, be)| be <= cur_s);
                    for &(bs, be) in &b_ivs[start_pos..] {
                        if bs >= a_e {
                            break;
                        }
                        if bs > cur_s {
                            result.push((chrom_s.clone(), cur_s, bs));
                        }
                        cur_s = cur_s.max(be);
                    }
                    if cur_s < a_e {
                        result.push((chrom_s, cur_s, a_e));
                    }
                }
            }
        }
    }
    result.sort_unstable_by(|x, y| x.0.cmp(&y.0).then(x.1.cmp(&y.1)));
    chrom_start_end_batch(result)
}

// ---------------------------------------------------------------------------
// PyO3 exports
// ---------------------------------------------------------------------------

#[pyfunction(name = "overlap")]
pub fn py_overlap<'py>(
    py: Python<'py>,
    a: &Bound<'_, PyAny>,
    b: &Bound<'_, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let batch_a = pyarrow_to_batch(py, a)?;
    let batch_b = pyarrow_to_batch(py, b)?;
    let result = py
        .detach(|| overlap_batches(&batch_a, &batch_b))
        .map_err(|e| PyIOError::new_err(e.to_string()))?;
    batch_to_pyarrow(py, result)
}

#[pyfunction(name = "nearest")]
pub fn py_nearest<'py>(
    py: Python<'py>,
    query: &Bound<'_, PyAny>,
    target: &Bound<'_, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let batch_q = pyarrow_to_batch(py, query)?;
    let batch_t = pyarrow_to_batch(py, target)?;
    let result = py
        .detach(|| nearest_batches(&batch_q, &batch_t))
        .map_err(|e| PyIOError::new_err(e.to_string()))?;
    batch_to_pyarrow(py, result)
}

#[pyfunction(name = "count_overlaps")]
pub fn py_count_overlaps<'py>(
    py: Python<'py>,
    a: &Bound<'_, PyAny>,
    b: &Bound<'_, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let batch_a = pyarrow_to_batch(py, a)?;
    let batch_b = pyarrow_to_batch(py, b)?;
    let result = py
        .detach(|| count_overlaps_batches(&batch_a, &batch_b))
        .map_err(|e| PyIOError::new_err(e.to_string()))?;
    batch_to_pyarrow(py, result)
}

#[pyfunction(name = "cluster")]
pub fn py_cluster<'py>(py: Python<'py>, a: &Bound<'_, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let batch = pyarrow_to_batch(py, a)?;
    let result = py
        .detach(|| cluster_batches(&batch))
        .map_err(|e| PyIOError::new_err(e.to_string()))?;
    batch_to_pyarrow(py, result)
}

#[pyfunction(name = "merge")]
pub fn py_merge<'py>(py: Python<'py>, a: &Bound<'_, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let batch = pyarrow_to_batch(py, a)?;
    let result = py
        .detach(|| merge_batches(&batch))
        .map_err(|e| PyIOError::new_err(e.to_string()))?;
    batch_to_pyarrow(py, result)
}

#[pyfunction(name = "subtract")]
pub fn py_subtract<'py>(
    py: Python<'py>,
    a: &Bound<'_, PyAny>,
    b: &Bound<'_, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let batch_a = pyarrow_to_batch(py, a)?;
    let batch_b = pyarrow_to_batch(py, b)?;
    let result = py
        .detach(|| subtract_batches(&batch_a, &batch_b))
        .map_err(|e| PyIOError::new_err(e.to_string()))?;
    batch_to_pyarrow(py, result)
}

#[pyfunction(name = "complement")]
#[pyo3(signature = (a, chrom_sizes=None))]
pub fn py_complement<'py>(
    py: Python<'py>,
    a: &Bound<'_, PyAny>,
    chrom_sizes: Option<std::collections::HashMap<String, i32>>,
) -> PyResult<Bound<'py, PyAny>> {
    let batch = pyarrow_to_batch(py, a)?;
    let result = py
        .detach(|| complement_batches(&batch, chrom_sizes.as_ref()))
        .map_err(|e| PyIOError::new_err(e.to_string()))?;
    batch_to_pyarrow(py, result)
}

#[pyfunction(name = "coverage")]
pub fn py_coverage<'py>(py: Python<'py>, a: &Bound<'_, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let batch = pyarrow_to_batch(py, a)?;
    let result = py
        .detach(|| coverage_batches(&batch))
        .map_err(|e| PyIOError::new_err(e.to_string()))?;
    batch_to_pyarrow(py, result)
}

#[pyfunction(name = "expand")]
#[pyo3(signature = (a, amount=0, start_amount=None, end_amount=None))]
pub fn py_expand<'py>(
    py: Python<'py>,
    a: &Bound<'_, PyAny>,
    amount: i32,
    start_amount: Option<i32>,
    end_amount: Option<i32>,
) -> PyResult<Bound<'py, PyAny>> {
    let batch = pyarrow_to_batch(py, a)?;
    let result = py
        .detach(|| expand_batches(&batch, amount, start_amount, end_amount))
        .map_err(|e| PyIOError::new_err(e.to_string()))?;
    batch_to_pyarrow(py, result)
}

#[pyfunction(name = "shift")]
pub fn py_shift<'py>(
    py: Python<'py>,
    a: &Bound<'_, PyAny>,
    amount: i32,
) -> PyResult<Bound<'py, PyAny>> {
    let batch = pyarrow_to_batch(py, a)?;
    let result = py
        .detach(|| shift_batches(&batch, amount))
        .map_err(|e| PyIOError::new_err(e.to_string()))?;
    batch_to_pyarrow(py, result)
}

#[pyfunction(name = "sort_bedframe")]
#[pyo3(signature = (a, natural_chrom_order=true))]
pub fn py_sort_bedframe<'py>(
    py: Python<'py>,
    a: &Bound<'_, PyAny>,
    natural_chrom_order: bool,
) -> PyResult<Bound<'py, PyAny>> {
    let batch = pyarrow_to_batch(py, a)?;
    let result = py
        .detach(|| sort_bedframe_batches(&batch, natural_chrom_order))
        .map_err(|e| PyIOError::new_err(e.to_string()))?;
    batch_to_pyarrow(py, result)
}

#[pyfunction(name = "flank")]
#[pyo3(signature = (a, width, start=true))]
pub fn py_flank<'py>(
    py: Python<'py>,
    a: &Bound<'_, PyAny>,
    width: i32,
    start: bool,
) -> PyResult<Bound<'py, PyAny>> {
    let batch = pyarrow_to_batch(py, a)?;
    let result = py
        .detach(|| flank_batches(&batch, width, start))
        .map_err(|e| PyIOError::new_err(e.to_string()))?;
    batch_to_pyarrow(py, result)
}

#[pyfunction(name = "set_width")]
#[pyo3(signature = (a, width, anchor = "start"))]
pub fn py_set_width<'py>(
    py: Python<'py>,
    a: &Bound<'_, PyAny>,
    width: i32,
    anchor: &str,
) -> PyResult<Bound<'py, PyAny>> {
    let batch = pyarrow_to_batch(py, a)?;
    let result = py
        .detach(|| set_width_batches(&batch, width, anchor))
        .map_err(|e| PyIOError::new_err(e.to_string()))?;
    batch_to_pyarrow(py, result)
}

#[pyfunction(name = "tile")]
pub fn py_tile<'py>(
    py: Python<'py>,
    a: &Bound<'_, PyAny>,
    width: i32,
) -> PyResult<Bound<'py, PyAny>> {
    let batch = pyarrow_to_batch(py, a)?;
    let result = py
        .detach(|| tile_batches(&batch, width))
        .map_err(|e| PyIOError::new_err(e.to_string()))?;
    batch_to_pyarrow(py, result)
}

#[pyfunction(name = "disjoin")]
pub fn py_disjoin<'py>(py: Python<'py>, a: &Bound<'_, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let batch = pyarrow_to_batch(py, a)?;
    let result = py
        .detach(|| disjoin_batches(&batch))
        .map_err(|e| PyIOError::new_err(e.to_string()))?;
    batch_to_pyarrow(py, result)
}

#[pyfunction(name = "intersect_ranges")]
pub fn py_intersect_ranges<'py>(
    py: Python<'py>,
    a: &Bound<'_, PyAny>,
    b: &Bound<'_, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let ba = pyarrow_to_batch(py, a)?;
    let bb = pyarrow_to_batch(py, b)?;
    let result = py
        .detach(|| intersect_ranges_batches(&ba, &bb))
        .map_err(|e| PyIOError::new_err(e.to_string()))?;
    batch_to_pyarrow(py, result)
}

#[pyfunction(name = "union_ranges")]
pub fn py_union_ranges<'py>(
    py: Python<'py>,
    a: &Bound<'_, PyAny>,
    b: &Bound<'_, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let ba = pyarrow_to_batch(py, a)?;
    let bb = pyarrow_to_batch(py, b)?;
    let result = py
        .detach(|| union_ranges_batches(&ba, &bb))
        .map_err(|e| PyIOError::new_err(e.to_string()))?;
    batch_to_pyarrow(py, result)
}

#[pyfunction(name = "setdiff_ranges")]
pub fn py_setdiff_ranges<'py>(
    py: Python<'py>,
    a: &Bound<'_, PyAny>,
    b: &Bound<'_, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let ba = pyarrow_to_batch(py, a)?;
    let bb = pyarrow_to_batch(py, b)?;
    let result = py
        .detach(|| setdiff_ranges_batches(&ba, &bb))
        .map_err(|e| PyIOError::new_err(e.to_string()))?;
    batch_to_pyarrow(py, result)
}
