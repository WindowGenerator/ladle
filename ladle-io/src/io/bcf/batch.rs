use std::sync::Arc;

use arrow::array::{
    Array, BooleanBuilder, Float32Builder, Int32Builder, LargeStringBuilder, RecordBatch,
};
use arrow::datatypes::{DataType, Field, Schema};
use noodles::vcf::header::record::value::map::info::Type as InfoType;
use noodles::vcf::variant::record::AlternateBases as _;
use noodles::vcf::variant::record::Ids as _;
use noodles::vcf::variant::record::Info as _;
use noodles::vcf::variant::record::ReferenceBases as _;
use noodles::vcf::variant::record::Samples as VcfSamples;
use noodles::vcf::variant::record::samples::Sample as VcfSample;
use pyo3::exceptions::{PyIOError, PyImportError};
use pyo3::prelude::*;

use super::record::PyRecord;
use crate::arrow_utils::{batch_to_pyarrow, pandas_to_batch, pyarrow_to_batch};
use crate::io::vcf::schema::{
    info_array_to_string, info_type_to_arrow, sample_value_to_string, vcf_base_schema,
};

fn build_bcf_schema_with_header(header: &noodles::vcf::Header) -> Arc<Schema> {
    let mut fields = vcf_base_schema();
    for (key, info_map) in header.infos() {
        let dtype = info_type_to_arrow(&info_map.ty());
        fields.push(Field::new(format!("INFO_{key}"), dtype, true));
    }
    let n_samples = header.sample_names().len();
    if n_samples > 0 {
        for (key, _) in header.formats() {
            fields.push(Field::new(format!("FMT_{key}"), DataType::LargeUtf8, true));
        }
    }
    Arc::new(Schema::new(fields))
}

fn build_bcf_batch_with_header(
    records: &[noodles::bcf::Record],
    header: &noodles::vcf::Header,
) -> Result<RecordBatch, arrow::error::ArrowError> {
    let n = records.len();
    let schema = build_bcf_schema_with_header(header);

    let info_keys: Vec<String> = header.infos().keys().cloned().collect();
    let fmt_keys: Vec<String> = header.formats().keys().cloned().collect();
    let n_samples = header.sample_names().len();

    let mut chroms = LargeStringBuilder::with_capacity(n, n * 5);
    let mut positions = Int32Builder::with_capacity(n);
    let mut ids = LargeStringBuilder::with_capacity(n, n * 5);
    let mut refs = LargeStringBuilder::with_capacity(n, n * 4);
    let mut alts = LargeStringBuilder::with_capacity(n, n * 4);
    let mut quals = Float32Builder::with_capacity(n);

    enum InfoBuilder {
        Int(Int32Builder),
        Float(Float32Builder),
        Bool(BooleanBuilder),
        Str(LargeStringBuilder),
    }

    let mut info_builders: Vec<InfoBuilder> = header
        .infos()
        .values()
        .map(|m| match m.ty() {
            InfoType::Integer => InfoBuilder::Int(Int32Builder::with_capacity(n)),
            InfoType::Float => InfoBuilder::Float(Float32Builder::with_capacity(n)),
            InfoType::Flag => InfoBuilder::Bool(BooleanBuilder::with_capacity(n)),
            _ => InfoBuilder::Str(LargeStringBuilder::with_capacity(n, n * 8)),
        })
        .collect();

    let mut fmt_builders: Vec<LargeStringBuilder> = if n_samples > 0 {
        fmt_keys
            .iter()
            .map(|_| LargeStringBuilder::with_capacity(n, n * n_samples * 4))
            .collect()
    } else {
        vec![]
    };

    for rec in records {
        // chrom — needs string_maps from header
        match rec.reference_sequence_name(header.string_maps()) {
            Ok(name) => chroms.append_value(name),
            Err(_) => chroms.append_value(""),
        }

        match rec.variant_start() {
            None => positions.append_null(),
            Some(Ok(pos)) => positions.append_value(pos.get() as i32),
            Some(Err(_)) => positions.append_null(),
        }

        let id_vec: Vec<String> = rec.ids().iter().map(|s| s.to_string()).collect();
        if id_vec.is_empty() {
            ids.append_null();
        } else {
            ids.append_value(id_vec.join(";"));
        }

        // ref bases
        {
            let bases: Vec<u8> = rec
                .reference_bases()
                .iter()
                .filter_map(|r| r.ok())
                .collect();
            refs.append_value(String::from_utf8_lossy(&bases));
        }

        // alt bases
        let alt_str: Vec<String> = rec
            .alternate_bases()
            .iter()
            .filter_map(|r| r.ok())
            .map(|s| s.to_string())
            .collect();
        if alt_str.is_empty() {
            alts.append_null();
        } else {
            alts.append_value(alt_str.join(","));
        }

        match rec.quality_score() {
            Ok(Some(v)) => quals.append_value(v),
            _ => quals.append_null(),
        }

        // INFO columns
        let rec_info = rec.info();
        let mut info_map: std::collections::HashMap<
            &str,
            noodles::vcf::variant::record::info::field::Value<'_>,
        > = std::collections::HashMap::new();
        for result in rec_info.iter(header) {
            if let Ok((k, Some(v))) = result {
                info_map.insert(k, v);
            }
        }

        for (idx, key) in info_keys.iter().enumerate() {
            let val = info_map.get(key.as_str());
            match &mut info_builders[idx] {
                InfoBuilder::Int(b) => {
                    use noodles::vcf::variant::record::info::field::value::Array as InfoArray;
                    match val {
                        Some(noodles::vcf::variant::record::info::field::Value::Integer(v)) => {
                            b.append_value(*v)
                        }
                        Some(noodles::vcf::variant::record::info::field::Value::Array(
                            InfoArray::Integer(arr),
                        )) => match arr.iter().next().and_then(|r| r.ok()).flatten() {
                            Some(v) => b.append_value(v),
                            None => b.append_null(),
                        },
                        _ => b.append_null(),
                    }
                }
                InfoBuilder::Float(b) => {
                    use noodles::vcf::variant::record::info::field::value::Array as InfoArray;
                    match val {
                        Some(noodles::vcf::variant::record::info::field::Value::Float(v)) => {
                            b.append_value(*v)
                        }
                        Some(noodles::vcf::variant::record::info::field::Value::Array(
                            InfoArray::Float(arr),
                        )) => match arr.iter().next().and_then(|r| r.ok()).flatten() {
                            Some(v) => b.append_value(v),
                            None => b.append_null(),
                        },
                        _ => b.append_null(),
                    }
                }
                InfoBuilder::Bool(b) => match val {
                    Some(noodles::vcf::variant::record::info::field::Value::Flag) => {
                        b.append_value(true)
                    }
                    _ => b.append_value(false),
                },
                InfoBuilder::Str(b) => match val {
                    Some(noodles::vcf::variant::record::info::field::Value::String(s)) => {
                        b.append_value(s.as_ref())
                    }
                    Some(noodles::vcf::variant::record::info::field::Value::Character(c)) => {
                        b.append_value(c.to_string());
                    }
                    Some(noodles::vcf::variant::record::info::field::Value::Array(arr)) => {
                        b.append_value(info_array_to_string(arr));
                    }
                    _ => b.append_null(),
                },
            }
        }

        // FORMAT columns — collect per-field tab-joined strings
        if n_samples > 0 && !fmt_keys.is_empty() {
            // fmt_cell[fmt_idx] accumulates one string per sample for this record
            let mut fmt_cell: Vec<Vec<String>> = fmt_keys
                .iter()
                .map(|_| Vec::with_capacity(n_samples))
                .collect();

            if let Ok(samples_data) = rec.samples() {
                for (sample_idx, sample) in VcfSamples::iter(&samples_data).enumerate() {
                    if sample_idx >= n_samples {
                        break;
                    }
                    for (fmt_idx, fmt_key) in fmt_keys.iter().enumerate() {
                        let s = match VcfSample::get(&sample, header, fmt_key.as_str()) {
                            Some(Ok(Some(v))) => sample_value_to_string(v),
                            _ => ".".to_string(),
                        };
                        fmt_cell[fmt_idx].push(s);
                    }
                }
            }

            for (fmt_idx, cell) in fmt_cell.iter().enumerate() {
                if cell.is_empty() {
                    fmt_builders[fmt_idx].append_null();
                } else {
                    fmt_builders[fmt_idx].append_value(cell.join("\t"));
                }
            }
        }
    }

    let mut columns: Vec<Arc<dyn Array>> = vec![
        Arc::new(chroms.finish()),
        Arc::new(positions.finish()),
        Arc::new(ids.finish()),
        Arc::new(refs.finish()),
        Arc::new(alts.finish()),
        Arc::new(quals.finish()),
    ];

    for b in info_builders {
        columns.push(match b {
            InfoBuilder::Int(mut b) => Arc::new(b.finish()),
            InfoBuilder::Float(mut b) => Arc::new(b.finish()),
            InfoBuilder::Bool(mut b) => Arc::new(b.finish()),
            InfoBuilder::Str(mut b) => Arc::new(b.finish()),
        });
    }

    for mut b in fmt_builders {
        columns.push(Arc::new(b.finish()));
    }

    RecordBatch::try_new(schema, columns)
}

// ---------------------------------------------------------------------------
// PyBcfRecordBatch
// ---------------------------------------------------------------------------

#[pyclass(name = "RecordBatch", module = "ladle.bcf")]
pub struct PyBcfRecordBatch {
    records: Option<Vec<noodles::bcf::Record>>,
    batch: RecordBatch,
}

impl PyBcfRecordBatch {
    pub fn try_new_with_header(
        records: Vec<noodles::bcf::Record>,
        header: &noodles::vcf::Header,
    ) -> Result<Self, arrow::error::ArrowError> {
        let batch = build_bcf_batch_with_header(&records, header)?;
        Ok(Self {
            records: Some(records),
            batch,
        })
    }
}

#[pymethods]
impl PyBcfRecordBatch {
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
        let arrow = self.to_arrow(py)?;
        let pl = py.import("polars").map_err(|_| {
            PyImportError::new_err("polars not installed — pip install ladle[polars]")
        })?;
        pl.getattr("from_arrow")?.call1((arrow,))
    }

    fn to_pandas<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        self.to_arrow(py)?.call_method0("to_pandas")
    }

    fn to_iterator(&self) -> PyResult<PyBcfBatchIterator> {
        match &self.records {
            Some(recs) => Ok(PyBcfBatchIterator {
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
            "RecordBatch(<{} records, {} columns>)",
            self.batch.num_rows(),
            self.batch.num_columns()
        )
    }
}

// ---------------------------------------------------------------------------
// PyBcfBatchIterator
// ---------------------------------------------------------------------------

#[pyclass(name = "RecordBatchIterator", module = "ladle.bcf")]
pub struct PyBcfBatchIterator {
    records: Vec<noodles::bcf::Record>,
    pos: usize,
}

#[pymethods]
impl PyBcfBatchIterator {
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
