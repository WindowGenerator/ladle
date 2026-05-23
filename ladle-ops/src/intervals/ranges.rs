use std::collections::HashMap;

use arrow::error::ArrowError;
use arrow::record_batch::RecordBatch;
use coitrees::{COITree, Interval, IntervalTree};

use super::helpers::chrom_start_end_batch;
use super::schema::{get_interval, resolve_interval_cols};

pub fn intersect_ranges_batches(
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

pub fn union_ranges_batches(a: &RecordBatch, b: &RecordBatch) -> Result<RecordBatch, ArrowError> {
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

pub fn setdiff_ranges_batches(a: &RecordBatch, b: &RecordBatch) -> Result<RecordBatch, ArrowError> {
    let a_cols = resolve_interval_cols(a.schema_ref())
        .map_err(|e| ArrowError::InvalidArgumentError(e.to_string()))?;
    let b_cols = resolve_interval_cols(b.schema_ref())
        .map_err(|e| ArrowError::InvalidArgumentError(e.to_string()))?;

    let mut b_sorted: HashMap<String, Vec<(i32, i32)>> = HashMap::new();
    for row in 0..b.num_rows() {
        if let Some((chrom, start, end)) = get_interval(b, &b_cols, row) {
            b_sorted
                .entry(chrom.to_string())
                .or_default()
                .push((start, end));
        }
    }
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
