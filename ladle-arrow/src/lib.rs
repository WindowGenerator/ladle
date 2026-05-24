use arrow::array::RecordBatch;
use arrow::compute::concat_batches;
use arrow_array::ffi_stream::{ArrowArrayStreamReader, FFI_ArrowArrayStream};
use arrow_array::{RecordBatchIterator as ArrowRecordBatchIterator, RecordBatchReader};
use pyo3::exceptions::{PyIOError, PyImportError};
use pyo3::prelude::*;

/// Arrow `RecordBatch` → `pyarrow.RecordBatch` via Arrow C Data Interface (zero-copy schema).
pub fn batch_to_pyarrow<'py>(py: Python<'py>, batch: RecordBatch) -> PyResult<Bound<'py, PyAny>> {
    let schema = batch.schema();
    let stream = FFI_ArrowArrayStream::new(Box::new(ArrowRecordBatchIterator::new(
        vec![Ok(batch)].into_iter(),
        schema,
    )));
    let stream_ptr = Box::into_raw(Box::new(stream)) as usize;
    let pa = py
        .import("pyarrow")
        .map_err(|_| PyImportError::new_err("pyarrow not installed — pip install ladle[arrow]"))?;
    let reader = pa
        .getattr("RecordBatchReader")?
        .getattr("_import_from_c")?
        .call1((stream_ptr,))?;
    reader.call_method0("read_next_batch")
}

pub fn batch_to_polars<'py>(py: Python<'py>, batch: RecordBatch) -> PyResult<Bound<'py, PyAny>> {
    let arrow = batch_to_pyarrow(py, batch)?;
    py.import("polars")
        .map_err(|_| PyImportError::new_err("polars not installed — pip install ladle[polars]"))?
        .getattr("from_arrow")?
        .call1((arrow,))
}

pub fn batch_to_pandas<'py>(py: Python<'py>, batch: RecordBatch) -> PyResult<Bound<'py, PyAny>> {
    let arrow = batch_to_pyarrow(py, batch)?;
    py.import("pandas")
        .map_err(|_| PyImportError::new_err("pandas not installed — pip install ladle[pandas]"))?;
    arrow.call_method0("to_pandas")
}

/// `pandas.DataFrame` → Arrow `RecordBatch` (via `pyarrow.Table.from_pandas`).
pub fn pandas_to_batch(py: Python<'_>, obj: &Bound<'_, PyAny>) -> PyResult<RecordBatch> {
    let pa = py
        .import("pyarrow")
        .map_err(|_| PyImportError::new_err("pyarrow not installed — pip install ladle[arrow]"))?;
    let table = pa.getattr("Table")?.call_method1("from_pandas", (obj,))?;
    pyarrow_to_batch(py, &table)
}

/// Any Python object with `__arrow_c_stream__` → Arrow `RecordBatch`.
/// Accepts pyarrow.RecordBatch, pyarrow.Table, polars.DataFrame, and anything
/// else that implements the Arrow C Stream Interface.
pub fn pyarrow_to_batch(py: Python<'_>, obj: &Bound<'_, PyAny>) -> PyResult<RecordBatch> {
    let pa = py
        .import("pyarrow")
        .map_err(|_| PyImportError::new_err("pyarrow not installed — pip install ladle[arrow]"))?;
    let reader_obj = pa
        .getattr("RecordBatchReader")?
        .call_method1("from_stream", (obj,))?;

    let raw = Box::into_raw(Box::new(FFI_ArrowArrayStream::empty()));
    let ptr = raw as usize;
    if let Err(e) = reader_obj.call_method1("_export_to_c", (ptr,)) {
        unsafe { drop(Box::from_raw(raw)) };
        return Err(e);
    }
    let stream_reader = unsafe { ArrowArrayStreamReader::from_raw(raw) }
        .map_err(|e| PyIOError::new_err(e.to_string()))?;
    let schema = stream_reader.schema();
    let batches: Vec<RecordBatch> = stream_reader
        .collect::<Result<_, _>>()
        .map_err(|e| PyIOError::new_err(e.to_string()))?;
    match batches.len() {
        0 => Ok(RecordBatch::new_empty(schema)),
        1 => Ok(batches.into_iter().next().unwrap()),
        _ => concat_batches(&schema, &batches).map_err(|e| PyIOError::new_err(e.to_string())),
    }
}
