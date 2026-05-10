use std::sync::Arc;

use arrow::array::{ArrayRef, Int32Array, StringArray, UInt32Array};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::error::ArrowError;
use arrow::record_batch::RecordBatch;

use super::schema::{get_interval, resolve_interval_cols};

pub fn prefixed_schema(a: &Schema, b: &Schema) -> Arc<Schema> {
    let fields: Vec<Field> = a
        .fields()
        .iter()
        .map(|f| Field::new(format!("a_{}", f.name()), f.data_type().clone(), f.is_nullable()))
        .chain(b.fields().iter().map(|f| {
            Field::new(format!("b_{}", f.name()), f.data_type().clone(), f.is_nullable())
        }))
        .collect();
    Arc::new(Schema::new(fields))
}

pub fn nearest_schema(a: &Schema, b: &Schema) -> Arc<Schema> {
    let mut fields: Vec<Field> =
        prefixed_schema(a, b).fields().iter().map(|f| f.as_ref().clone()).collect();
    fields.push(Field::new("distance", DataType::Int64, true));
    Arc::new(Schema::new(fields))
}

pub fn take_rows(batch: &RecordBatch, indices: &UInt32Array) -> Result<Vec<ArrayRef>, ArrowError> {
    let idx = arrow::array::cast::as_primitive_array::<arrow::datatypes::UInt32Type>(indices);
    batch
        .columns()
        .iter()
        .map(|col| arrow::compute::take(col.as_ref(), idx, None))
        .collect()
}

pub fn sorted_intervals(
    batch: &RecordBatch,
) -> Result<Vec<(String, i32, i32, usize)>, ArrowError> {
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

pub fn chrom_start_end_batch(rows: Vec<(String, i32, i32)>) -> Result<RecordBatch, ArrowError> {
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
