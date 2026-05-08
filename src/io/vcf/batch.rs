use arrow::array::RecordBatch;
use pyo3::exceptions::{PyImportError, PyIOError};
use pyo3::prelude::*;

use crate::arrow_utils::{batch_to_pyarrow, pandas_to_batch, pyarrow_to_batch};
use super::record::PyRecord;
use super::schema::{build_vcf_batch, build_vcf_batch_with_header};

// ---------------------------------------------------------------------------
// PyVcfRecordBatch
// ---------------------------------------------------------------------------

#[pyclass(name = "RecordBatch", module = "ladle.vcf")]
pub struct PyVcfRecordBatch {
    records: Option<Vec<noodles::vcf::Record>>,
    batch: RecordBatch,
}

impl PyVcfRecordBatch {
    pub fn try_new(records: Vec<noodles::vcf::Record>) -> Result<Self, arrow::error::ArrowError> {
        let batch = build_vcf_batch(&records)?;
        Ok(Self { records: Some(records), batch })
    }

    pub fn try_new_with_header(
        records: Vec<noodles::vcf::Record>,
        header: &noodles::vcf::Header,
    ) -> Result<Self, arrow::error::ArrowError> {
        let batch = build_vcf_batch_with_header(&records, header)?;
        Ok(Self { records: Some(records), batch })
    }
}

#[pymethods]
impl PyVcfRecordBatch {
    #[staticmethod]
    fn from_arrow(py: Python<'_>, obj: &Bound<'_, PyAny>) -> PyResult<Self> {
        Ok(Self { records: None, batch: pyarrow_to_batch(py, obj)? })
    }

    #[staticmethod]
    fn from_polars(py: Python<'_>, obj: &Bound<'_, PyAny>) -> PyResult<Self> {
        Ok(Self { records: None, batch: pyarrow_to_batch(py, obj)? })
    }

    #[staticmethod]
    fn from_pandas(py: Python<'_>, obj: &Bound<'_, PyAny>) -> PyResult<Self> {
        Ok(Self { records: None, batch: pandas_to_batch(py, obj)? })
    }

    fn to_arrow<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        batch_to_pyarrow(py, self.batch.clone())
    }

    fn to_polars<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let arrow = self.to_arrow(py)?;
        let pl = py.import("polars").map_err(|_| {
            PyImportError::new_err("polars not installed — pip install ladle[polars]")
        })?;
        pl.getattr("from_arrow")?.call1((arrow,))
    }

    fn to_pandas<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        self.to_arrow(py)?.call_method0("to_pandas")
    }

    fn to_iterator(&self) -> PyResult<PyVcfBatchIterator> {
        match &self.records {
            Some(recs) => Ok(PyVcfBatchIterator { records: recs.clone(), pos: 0 }),
            None => Err(PyIOError::new_err(
                "to_iterator() is not available on a RecordBatch created from external data (from_arrow/from_polars/from_pandas)"
            )),
        }
    }

    fn __len__(&self) -> usize {
        self.batch.num_rows()
    }

    fn __repr__(&self) -> String {
        format!("RecordBatch(<{} records, {} columns>)", self.batch.num_rows(), self.batch.num_columns())
    }
}

// ---------------------------------------------------------------------------
// PyVcfBatchIterator
// ---------------------------------------------------------------------------

#[pyclass(name = "RecordBatchIterator", module = "ladle.vcf")]
pub struct PyVcfBatchIterator {
    records: Vec<noodles::vcf::Record>,
    pos: usize,
}

#[pymethods]
impl PyVcfBatchIterator {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(&mut self) -> Option<PyRecord> {
        if self.pos < self.records.len() {
            let rec = self.records[self.pos].clone();
            self.pos += 1;
            Some(PyRecord::from(rec))
        } else {
            None
        }
    }
}
