use std::sync::Arc;

use arrow::array::{
    Array, Int32Builder, LargeBinaryBuilder, RecordBatch, UInt8Builder, UInt16Builder,
};
use arrow::datatypes::{DataType, Field, Schema};
use bstr::ByteSlice;
use pyo3::exceptions::PyIOError;
use pyo3::prelude::*;

use super::record::PyRecord;
use crate::arrow_utils::{batch_to_pandas, batch_to_polars, batch_to_pyarrow, pandas_to_batch, pyarrow_to_batch};

fn sam_schema() -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new("name", DataType::LargeBinary, true),
        Field::new("flags", DataType::UInt16, false),
        Field::new("reference_sequence_name", DataType::LargeBinary, true),
        Field::new("alignment_start", DataType::Int32, true),
        Field::new("mapping_quality", DataType::UInt8, true),
        Field::new("cigar", DataType::LargeBinary, false),
        Field::new("mate_reference_sequence_name", DataType::LargeBinary, true),
        Field::new("mate_alignment_start", DataType::Int32, true),
        Field::new("template_length", DataType::Int32, false),
        Field::new("sequence", DataType::LargeBinary, false),
        Field::new("quality_scores", DataType::LargeBinary, false),
    ]))
}

fn build_sam_batch(
    records: &[noodles::sam::Record],
) -> Result<RecordBatch, arrow::error::ArrowError> {
    let n = records.len();
    let mut names = LargeBinaryBuilder::with_capacity(n, n * 10);
    let mut flags = UInt16Builder::with_capacity(n);
    let mut ref_names = LargeBinaryBuilder::with_capacity(n, n * 5);
    let mut aln_starts = Int32Builder::with_capacity(n);
    let mut mapqs = UInt8Builder::with_capacity(n);
    let mut cigars = LargeBinaryBuilder::with_capacity(n, n * 8);
    let mut mate_refs = LargeBinaryBuilder::with_capacity(n, n * 5);
    let mut mate_alns = Int32Builder::with_capacity(n);
    let mut tlen = Int32Builder::with_capacity(n);
    let mut seqs = LargeBinaryBuilder::with_capacity(n, n * 150);
    let mut quals = LargeBinaryBuilder::with_capacity(n, n * 150);

    for rec in records {
        names.append_option(rec.name().map(|n| n.as_bytes()));

        match rec.flags() {
            Ok(f) => flags.append_value(u16::from(f)),
            Err(_) => flags.append_value(0),
        }

        ref_names.append_option(rec.reference_sequence_name().map(|n| n.as_bytes()));

        match rec.alignment_start() {
            None => aln_starts.append_null(),
            Some(Ok(pos)) => aln_starts.append_value(pos.get() as i32),
            Some(Err(_)) => aln_starts.append_null(),
        }

        match rec.mapping_quality() {
            None => mapqs.append_null(),
            Some(Ok(mq)) => mapqs.append_value(mq.get()),
            Some(Err(_)) => mapqs.append_null(),
        }

        cigars.append_value(rec.cigar().as_ref());

        mate_refs.append_option(rec.mate_reference_sequence_name().map(|n| n.as_bytes()));

        match rec.mate_alignment_start() {
            None => mate_alns.append_null(),
            Some(Ok(pos)) => mate_alns.append_value(pos.get() as i32),
            Some(Err(_)) => mate_alns.append_null(),
        }

        match rec.template_length() {
            Ok(v) => tlen.append_value(v),
            Err(_) => tlen.append_value(0),
        }

        seqs.append_value(rec.sequence().as_ref());
        quals.append_value(rec.quality_scores().as_ref());
    }

    let columns: Vec<Arc<dyn Array>> = vec![
        Arc::new(names.finish()),
        Arc::new(flags.finish()),
        Arc::new(ref_names.finish()),
        Arc::new(aln_starts.finish()),
        Arc::new(mapqs.finish()),
        Arc::new(cigars.finish()),
        Arc::new(mate_refs.finish()),
        Arc::new(mate_alns.finish()),
        Arc::new(tlen.finish()),
        Arc::new(seqs.finish()),
        Arc::new(quals.finish()),
    ];

    RecordBatch::try_new(sam_schema(), columns)
}

// ---------------------------------------------------------------------------
// PySamRecordBatch
// ---------------------------------------------------------------------------

#[pyclass(name = "RecordBatch", module = "ladle.sam")]
pub struct PySamRecordBatch {
    records: Option<Vec<noodles::sam::Record>>,
    batch: RecordBatch,
}

impl PySamRecordBatch {
    pub fn try_new(records: Vec<noodles::sam::Record>) -> Result<Self, arrow::error::ArrowError> {
        let batch = build_sam_batch(&records)?;
        Ok(Self {
            records: Some(records),
            batch,
        })
    }
}

#[pymethods]
impl PySamRecordBatch {
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

    fn to_iterator(&self) -> PyResult<PySamBatchIterator> {
        match &self.records {
            Some(recs) => Ok(PySamBatchIterator {
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
            "RecordBatch(<{} records, 11 columns>)",
            self.batch.num_rows()
        )
    }
}

// ---------------------------------------------------------------------------
// PySamBatchIterator
// ---------------------------------------------------------------------------

#[pyclass(name = "RecordBatchIterator", module = "ladle.sam")]
pub struct PySamBatchIterator {
    records: Vec<noodles::sam::Record>,
    pos: usize,
}

#[pymethods]
impl PySamBatchIterator {
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
