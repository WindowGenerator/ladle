use arrow::array::{Array, Int32Array, Int64Array, LargeStringArray, StringArray, StringViewArray};
use arrow::datatypes::{DataType, Schema};
use arrow::record_batch::RecordBatch;
use pyo3::PyResult;
use pyo3::exceptions::PyValueError;

pub struct IntervalCols {
    pub chrom: usize,
    pub start: usize,
    pub end: usize,
}

pub fn resolve_interval_cols(schema: &Schema) -> PyResult<IntervalCols> {
    let chrom = find_col(schema, &["chrom", "contig", "chr"])?;
    let start = find_col(schema, &["start", "pos"])?;
    let end = find_col(schema, &["end", "stop"])?;
    Ok(IntervalCols { chrom, start, end })
}

pub fn resolve_on_cols(schema: &Schema, names: &[String]) -> PyResult<Vec<usize>> {
    names
        .iter()
        .map(|name| {
            schema
                .column_with_name(name)
                .map(|(idx, _)| idx)
                .ok_or_else(|| PyValueError::new_err(format!("on_cols column not found: '{name}'")))
        })
        .collect()
}

pub fn group_key(
    batch: &RecordBatch,
    cols: &IntervalCols,
    on_col_indices: &[usize],
    row: usize,
) -> Option<Vec<String>> {
    let chrom = extract_str(batch.column(cols.chrom).as_ref(), row)?;
    let mut key = vec![chrom.to_string()];
    for &idx in on_col_indices {
        key.push(extract_str(batch.column(idx).as_ref(), row)?.to_string());
    }
    Some(key)
}

fn find_col(schema: &Schema, candidates: &[&str]) -> PyResult<usize> {
    for name in candidates {
        if let Some((idx, _)) = schema.column_with_name(name) {
            return Ok(idx);
        }
    }
    Err(PyValueError::new_err(format!(
        "could not find interval column; tried: {:?}",
        candidates
    )))
}

/// Extract (chrom_str, start_i32, end_i32) for row `i`.
/// Returns None if any value is null or out of i32 range.
pub fn get_interval<'a>(
    batch: &'a RecordBatch,
    cols: &IntervalCols,
    row: usize,
) -> Option<(&'a str, i32, i32)> {
    let chrom = extract_str(batch.column(cols.chrom).as_ref(), row)?;
    let start = extract_i32(batch.column(cols.start).as_ref(), row)?;
    let end = extract_i32(batch.column(cols.end).as_ref(), row)?;
    Some((chrom, start, end))
}

fn extract_i32(array: &dyn Array, row: usize) -> Option<i32> {
    if array.is_null(row) {
        return None;
    }
    match array.data_type() {
        DataType::Int32 => array
            .as_any()
            .downcast_ref::<Int32Array>()
            .map(|a| a.value(row)),
        DataType::Int64 => {
            let v = array.as_any().downcast_ref::<Int64Array>()?.value(row);
            i32::try_from(v).ok()
        }
        _ => None,
    }
}

fn extract_str(array: &dyn Array, row: usize) -> Option<&str> {
    if array.is_null(row) {
        return None;
    }
    match array.data_type() {
        DataType::LargeUtf8 => array
            .as_any()
            .downcast_ref::<LargeStringArray>()
            .map(|a| a.value(row)),
        DataType::Utf8 => array
            .as_any()
            .downcast_ref::<StringArray>()
            .map(|a| a.value(row)),
        DataType::Utf8View => array
            .as_any()
            .downcast_ref::<StringViewArray>()
            .map(|a| a.value(row)),
        _ => None,
    }
}
