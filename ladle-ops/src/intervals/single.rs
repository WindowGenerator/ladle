use std::collections::HashMap;
use std::sync::Arc;

use arrow::array::{ArrayRef, Int32Array, UInt32Array};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::error::ArrowError;
use arrow::record_batch::RecordBatch;

use coitrees::{COITree, Interval, IntervalTree};

use super::helpers::{chrom_start_end_batch, sorted_intervals, take_rows};
use super::schema::{get_interval, resolve_interval_cols};

pub fn cluster_batches(batch: &RecordBatch) -> Result<RecordBatch, ArrowError> {
    let sorted = sorted_intervals(batch)?;

    let mut assignments: Vec<(usize, u32)> = Vec::with_capacity(sorted.len());
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

pub fn merge_batches(batch: &RecordBatch) -> Result<RecordBatch, ArrowError> {
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

pub fn complement_batches(
    batch: &RecordBatch,
    chrom_sizes: Option<&HashMap<String, i32>>,
) -> Result<RecordBatch, ArrowError> {
    let sorted = sorted_intervals(batch)?;
    let mut gaps: Vec<(String, i32, i32)> = Vec::new();
    let mut cur_chrom = String::new();
    let mut cur_end: i32 = 0;

    for (chrom, start, end, _) in &sorted {
        if *chrom != cur_chrom {
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
    if let Some(sizes) = chrom_sizes
        && let Some(&sz) = sizes.get(&cur_chrom)
        && cur_end < sz
    {
        gaps.push((cur_chrom, cur_end, sz));
    }
    chrom_start_end_batch(gaps)
}

pub fn coverage_batches(batch: &RecordBatch) -> Result<RecordBatch, ArrowError> {
    use arrow::array::{StringArray, UInt32Array as UA};

    let cols = resolve_interval_cols(batch.schema_ref())
        .map_err(|e| ArrowError::InvalidArgumentError(e.to_string()))?;

    let mut chrom_events: HashMap<String, Vec<(i32, i32)>> = HashMap::new();
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

pub fn expand_batches(
    batch: &RecordBatch,
    amount: i32,
    start_amount: Option<i32>,
    end_amount: Option<i32>,
) -> Result<RecordBatch, ArrowError> {
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

pub fn shift_batches(batch: &RecordBatch, amount: i32) -> Result<RecordBatch, ArrowError> {
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

pub fn sort_bedframe_batches(
    batch: &RecordBatch,
    natural: bool,
) -> Result<RecordBatch, ArrowError> {
    use arrow::array::StringArray;
    use arrow::compute::{SortColumn, SortOptions, lexsort_to_indices};

    let cols = resolve_interval_cols(batch.schema_ref())
        .map_err(|e| ArrowError::InvalidArgumentError(e.to_string()))?;

    let opts = SortOptions {
        descending: false,
        nulls_first: false,
    };

    let sort_indices = if natural {
        let chrom_col = batch
            .column(cols.chrom)
            .as_any()
            .downcast_ref::<StringArray>()
            .ok_or_else(|| ArrowError::InvalidArgumentError("chrom not Utf8".into()))?;

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

pub fn flank_batches(
    batch: &RecordBatch,
    width: i32,
    start: bool,
) -> Result<RecordBatch, ArrowError> {
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
        let ns = (0..batch.num_rows())
            .map(|i| (old_start.value(i) - width).max(1))
            .collect();
        let ne = (0..batch.num_rows()).map(|i| old_start.value(i)).collect();
        (ns, ne)
    } else {
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

pub fn set_width_batches(
    batch: &RecordBatch,
    width: i32,
    anchor: &str,
) -> Result<RecordBatch, ArrowError> {
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

pub fn tile_batches(batch: &RecordBatch, width: i32) -> Result<RecordBatch, ArrowError> {
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
    let new_start: Int32Array = tile_starts.into_iter().collect();
    let new_end: Int32Array = tile_ends.into_iter().collect();
    columns[cols.start] = Arc::new(new_start) as ArrayRef;
    columns[cols.end] = Arc::new(new_end) as ArrayRef;
    RecordBatch::try_new(batch.schema(), columns)
}

pub fn subtract_batches(a: &RecordBatch, b: &RecordBatch) -> Result<RecordBatch, ArrowError> {
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

pub fn disjoin_batches(batch: &RecordBatch) -> Result<RecordBatch, ArrowError> {
    let sorted = sorted_intervals(batch)?;
    let mut result: Vec<(String, i32, i32)> = Vec::new();

    let mut i = 0;
    while i < sorted.len() {
        let chrom = sorted[i].0.clone();
        let j = sorted[i..]
            .iter()
            .position(|(c, _, _, _)| c != &chrom)
            .map(|p| i + p)
            .unwrap_or(sorted.len());

        let mut pts: Vec<i32> = Vec::new();
        for row in &sorted[i..j] {
            pts.push(row.1);
            pts.push(row.2);
        }
        pts.sort_unstable();
        pts.dedup();

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
