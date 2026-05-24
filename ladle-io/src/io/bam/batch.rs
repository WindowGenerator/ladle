use std::sync::Arc;

use arrow::array::{
    Array, Int32Builder, LargeBinaryBuilder, RecordBatch, UInt8Builder, UInt16Builder,
};
use arrow::datatypes::{DataType, Field, Schema};
use bstr::ByteSlice;
use pyo3::exceptions::PyIOError;
use pyo3::prelude::*;

use super::record::PyRecord;
use crate::arrow_utils::{
    batch_to_pandas, batch_to_polars, batch_to_pyarrow, pandas_to_batch, pyarrow_to_batch,
};

fn bam_schema() -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new("name", DataType::LargeBinary, true),
        Field::new("flags", DataType::UInt16, false),
        Field::new("reference_sequence_id", DataType::Int32, true),
        Field::new("alignment_start", DataType::Int32, true),
        Field::new("mapping_quality", DataType::UInt8, true),
        Field::new("cigar", DataType::LargeBinary, false),
        Field::new("mate_reference_sequence_id", DataType::Int32, true),
        Field::new("mate_alignment_start", DataType::Int32, true),
        Field::new("template_length", DataType::Int32, false),
        Field::new("sequence", DataType::LargeBinary, false),
        Field::new("quality_scores", DataType::LargeBinary, false),
    ]))
}

fn build_bam_batch(
    records: &[noodles::bam::Record],
) -> Result<RecordBatch, arrow::error::ArrowError> {
    let n = records.len();
    let mut names = LargeBinaryBuilder::with_capacity(n, n * 10);
    let mut flags = UInt16Builder::with_capacity(n);
    let mut ref_ids = Int32Builder::with_capacity(n);
    let mut aln_starts = Int32Builder::with_capacity(n);
    let mut mapqs = UInt8Builder::with_capacity(n);
    let mut cigars = LargeBinaryBuilder::with_capacity(n, n * 8);
    let mut mate_ref_ids = Int32Builder::with_capacity(n);
    let mut mate_alns = Int32Builder::with_capacity(n);
    let mut tlen = Int32Builder::with_capacity(n);
    let mut seqs = LargeBinaryBuilder::with_capacity(n, n * 150);
    let mut quals = LargeBinaryBuilder::with_capacity(n, n * 150);

    for rec in records {
        names.append_option(rec.name().map(|nm| nm.as_bytes()));

        flags.append_value(u16::from(rec.flags()));

        match rec.reference_sequence_id() {
            None => ref_ids.append_null(),
            Some(Ok(id)) => ref_ids.append_value(id as i32),
            Some(Err(_)) => ref_ids.append_null(),
        }

        match rec.alignment_start() {
            None => aln_starts.append_null(),
            Some(Ok(pos)) => aln_starts.append_value(pos.get() as i32),
            Some(Err(_)) => aln_starts.append_null(),
        }

        match rec.mapping_quality() {
            None => mapqs.append_null(),
            Some(mq) => mapqs.append_value(mq.get()),
        }

        cigars.append_value(rec.cigar().as_ref());

        match rec.mate_reference_sequence_id() {
            None => mate_ref_ids.append_null(),
            Some(Ok(id)) => mate_ref_ids.append_value(id as i32),
            Some(Err(_)) => mate_ref_ids.append_null(),
        }

        match rec.mate_alignment_start() {
            None => mate_alns.append_null(),
            Some(Ok(pos)) => mate_alns.append_value(pos.get() as i32),
            Some(Err(_)) => mate_alns.append_null(),
        }

        tlen.append_value(rec.template_length());
        seqs.append_value(rec.sequence().as_ref());
        quals.append_value(rec.quality_scores().as_ref());
    }

    let columns: Vec<Arc<dyn Array>> = vec![
        Arc::new(names.finish()),
        Arc::new(flags.finish()),
        Arc::new(ref_ids.finish()),
        Arc::new(aln_starts.finish()),
        Arc::new(mapqs.finish()),
        Arc::new(cigars.finish()),
        Arc::new(mate_ref_ids.finish()),
        Arc::new(mate_alns.finish()),
        Arc::new(tlen.finish()),
        Arc::new(seqs.finish()),
        Arc::new(quals.finish()),
    ];

    RecordBatch::try_new(bam_schema(), columns)
}

// ---------------------------------------------------------------------------
// PyBamRecordBatch
// ---------------------------------------------------------------------------

#[pyclass(name = "RecordBatch", module = "ladle.bam")]
pub struct PyBamRecordBatch {
    records: Option<Vec<noodles::bam::Record>>,
    batch: RecordBatch,
}

impl PyBamRecordBatch {
    pub fn try_new(records: Vec<noodles::bam::Record>) -> Result<Self, arrow::error::ArrowError> {
        let batch = build_bam_batch(&records)?;
        Ok(Self {
            records: Some(records),
            batch,
        })
    }
}

#[pymethods]
impl PyBamRecordBatch {
    // Accepts pyarrow.RecordBatch, pyarrow.Table, or any object with __arrow_c_stream__.
    #[staticmethod]
    fn from_arrow(py: Python<'_>, obj: &Bound<'_, PyAny>) -> PyResult<Self> {
        let batch = pyarrow_to_batch(py, obj)?;
        Ok(Self {
            records: None,
            batch,
        })
    }

    // polars.DataFrame also supports __arrow_c_stream__, so pass directly.
    #[staticmethod]
    fn from_polars(py: Python<'_>, obj: &Bound<'_, PyAny>) -> PyResult<Self> {
        let batch = pyarrow_to_batch(py, obj)?;
        Ok(Self {
            records: None,
            batch,
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

    fn to_iterator(&self) -> PyResult<PyBamBatchIterator> {
        match &self.records {
            Some(recs) => Ok(PyBamBatchIterator {
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
// PyBamBatchIterator
// ---------------------------------------------------------------------------

#[pyclass(name = "RecordBatchIterator", module = "ladle.bam")]
pub struct PyBamBatchIterator {
    records: Vec<noodles::bam::Record>,
    pos: usize,
}

#[pymethods]
impl PyBamBatchIterator {
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
