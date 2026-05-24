use std::sync::Arc;

use arrow::array::{ArrayRef, LargeBinaryBuilder, RecordBatch};
use arrow::datatypes::{DataType, Field, Schema};
use pyo3::exceptions::PyIOError;
use pyo3::prelude::*;

use super::record::PyRecord;
use crate::arrow_utils::{
    batch_to_pandas, batch_to_polars, batch_to_pyarrow, pandas_to_batch, pyarrow_to_batch,
};

fn fastq_schema() -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new("name", DataType::LargeBinary, false),
        Field::new("description", DataType::LargeBinary, true),
        Field::new("sequence", DataType::LargeBinary, false),
        Field::new("quality_scores", DataType::LargeBinary, false),
    ]))
}

fn build_fastq_batch(
    records: &[noodles::fastq::Record],
) -> Result<RecordBatch, arrow::error::ArrowError> {
    let n = records.len();
    let mut names = LargeBinaryBuilder::with_capacity(n, n * 20);
    let mut descriptions = LargeBinaryBuilder::with_capacity(n, n * 10);
    let mut sequences = LargeBinaryBuilder::with_capacity(n, n * 150);
    let mut quality_scores = LargeBinaryBuilder::with_capacity(n, n * 150);

    for rec in records {
        names.append_value(rec.name());

        let desc = rec.description();
        if desc.is_empty() {
            descriptions.append_null();
        } else {
            descriptions.append_value(desc);
        }

        sequences.append_value(rec.sequence());
        quality_scores.append_value(rec.quality_scores());
    }

    let columns: Vec<ArrayRef> = vec![
        Arc::new(names.finish()),
        Arc::new(descriptions.finish()),
        Arc::new(sequences.finish()),
        Arc::new(quality_scores.finish()),
    ];

    RecordBatch::try_new(fastq_schema(), columns)
}

// ---------------------------------------------------------------------------
// PyFastqRecordBatch
// ---------------------------------------------------------------------------

#[pyclass(name = "RecordBatch", module = "ladle.fastq")]
pub struct PyFastqRecordBatch {
    records: Option<Vec<noodles::fastq::Record>>,
    batch: RecordBatch,
}

impl PyFastqRecordBatch {
    pub fn try_new(records: Vec<noodles::fastq::Record>) -> Result<Self, arrow::error::ArrowError> {
        let batch = build_fastq_batch(&records)?;
        Ok(Self {
            records: Some(records),
            batch,
        })
    }
}

#[pymethods]
impl PyFastqRecordBatch {
    #[staticmethod]
    fn from_arrow(py: Python<'_>, obj: &Bound<'_, PyAny>) -> PyResult<Self> {
        Ok(Self {
            records: None,
            batch: pyarrow_to_batch(py, obj)?,
        })
    }

    #[staticmethod]
    fn from_polars(py: Python<'_>, obj: &Bound<'_, PyAny>) -> PyResult<Self> {
        Ok(Self {
            records: None,
            batch: pyarrow_to_batch(py, obj)?,
        })
    }

    #[staticmethod]
    fn from_pandas(py: Python<'_>, obj: &Bound<'_, PyAny>) -> PyResult<Self> {
        Ok(Self {
            records: None,
            batch: pandas_to_batch(py, obj)?,
        })
    }

    fn to_arrow<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        batch_to_pyarrow(py, self.batch.clone())
    }

    fn to_polars<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        batch_to_polars(py, self.batch.clone())
    }

    fn to_pandas<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        batch_to_pandas(py, self.batch.clone())
    }

    fn to_iterator(&self) -> PyResult<PyFastqBatchIterator> {
        match &self.records {
            Some(recs) => Ok(PyFastqBatchIterator {
                records: recs.clone(),
                pos: 0,
            }),
            None => Err(PyIOError::new_err(
                "to_iterator() is not available on a RecordBatch created from external data (from_arrow/from_polars/from_pandas)",
            )),
        }
    }

    fn __len__(&self) -> usize {
        self.batch.num_rows()
    }

    fn __repr__(&self) -> String {
        format!(
            "RecordBatch(<{} records, 4 columns>)",
            self.batch.num_rows()
        )
    }
}

// ---------------------------------------------------------------------------
// PyFastqBatchIterator
// ---------------------------------------------------------------------------

#[pyclass(name = "RecordBatchIterator", module = "ladle.fastq")]
pub struct PyFastqBatchIterator {
    records: Vec<noodles::fastq::Record>,
    pos: usize,
}

#[pymethods]
impl PyFastqBatchIterator {
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
