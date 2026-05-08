use noodles::vcf::variant::record::info::field::Value;
use noodles::vcf::variant::record::info::field::value::Array;
use noodles::vcf::variant::record::AlternateBases as _;
use noodles::vcf::variant::record::Filters as _;
use noodles::vcf::variant::record::Ids as _;
use noodles::vcf::variant::record::Info as _;
use pyo3::exceptions::PyIOError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyString};

use crate::io::core::PyPosition;
use super::header::PyHeader;

#[pyclass(name = "Record", module = "ladle.vcf", from_py_object)]
#[derive(Clone)]
pub struct PyRecord {
    pub inner: noodles::vcf::Record,
}

impl From<noodles::vcf::Record> for PyRecord {
    fn from(inner: noodles::vcf::Record) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyRecord {
    fn reference_sequence_name<'py>(&self, py: Python<'py>) -> Bound<'py, PyString> {
        PyString::new(py, self.inner.reference_sequence_name())
    }

    fn variant_start(&self) -> PyResult<Option<PyPosition>> {
        match self.inner.variant_start() {
            None => Ok(None),
            Some(Ok(pos)) => Ok(Some(PyPosition::from(pos))),
            Some(Err(e)) => Err(PyIOError::new_err(e.to_string())),
        }
    }

    fn ids<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyList>> {
        let items: Vec<Bound<'_, PyString>> = self
            .inner
            .ids()
            .iter()
            .map(|id| PyString::new(py, id))
            .collect();
        PyList::new(py, items)
    }

    fn reference_bases<'py>(&self, py: Python<'py>) -> Bound<'py, PyString> {
        PyString::new(py, self.inner.reference_bases())
    }

    fn alternate_bases<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyList>> {
        let alts: Vec<Bound<'_, PyString>> = self
            .inner
            .alternate_bases()
            .iter()
            .map(|r| r.map(|s| PyString::new(py, s)).map_err(|e| PyIOError::new_err(e.to_string())))
            .collect::<PyResult<_>>()?;
        PyList::new(py, alts)
    }

    fn quality_score(&self) -> PyResult<Option<f32>> {
        match self.inner.quality_score() {
            None => Ok(None),
            Some(Ok(v)) => Ok(Some(v)),
            Some(Err(e)) => Err(PyIOError::new_err(e.to_string())),
        }
    }

    fn filters<'py>(&self, py: Python<'py>, header: &PyHeader) -> PyResult<Bound<'py, PyList>> {
        let filters: Vec<Bound<'_, PyString>> = self
            .inner
            .filters()
            .iter(&header.inner)
            .map(|r| r.map(|s| PyString::new(py, s)).map_err(|e| PyIOError::new_err(e.to_string())))
            .collect::<PyResult<_>>()?;
        PyList::new(py, filters)
    }

    fn info<'py>(&self, py: Python<'py>, header: &PyHeader) -> PyResult<Bound<'py, PyDict>> {
        let dict = PyDict::new(py);
        for result in self.inner.info().iter(&header.inner) {
            let (key, value) = result.map_err(|e| PyIOError::new_err(e.to_string()))?;
            let py_val = match value {
                None => py.None().into_bound(py),
                Some(v) => info_value_to_py(py, v)?,
            };
            dict.set_item(key, py_val)?;
        }
        Ok(dict)
    }

    fn __repr__(&self) -> String {
        let chrom = self.inner.reference_sequence_name();
        let pos = self
            .inner
            .variant_start()
            .and_then(|r| r.ok())
            .map(|p| p.get().to_string())
            .unwrap_or_else(|| "?".to_string());
        format!("Record(chrom={chrom:?}, pos={pos})")
    }
}

fn info_value_to_py<'py>(py: Python<'py>, value: Value<'_>) -> PyResult<Bound<'py, PyAny>> {
    Ok(match value {
        Value::Integer(v) => v.into_pyobject(py)?.into_any(),
        Value::Float(v) => v.into_pyobject(py)?.into_any(),
        Value::Flag => true.into_pyobject(py)?.to_owned().into_any(),
        Value::Character(c) => {
            let s = c.to_string();
            PyString::new(py, &s).into_any()
        }
        Value::String(s) => PyString::new(py, s.as_ref()).into_any(),
        Value::Array(arr) => info_array_to_py(py, arr)?.into_any(),
    })
}

fn info_array_to_py<'py>(py: Python<'py>, arr: Array<'_>) -> PyResult<Bound<'py, PyList>> {
    match arr {
        Array::Integer(values) => {
            let items: Vec<Option<i32>> = values
                .iter()
                .map(|r| r.map_err(|e| PyIOError::new_err(e.to_string())))
                .collect::<PyResult<_>>()?;
            PyList::new(py, items)
        }
        Array::Float(values) => {
            let items: Vec<Option<f32>> = values
                .iter()
                .map(|r| r.map_err(|e| PyIOError::new_err(e.to_string())))
                .collect::<PyResult<_>>()?;
            PyList::new(py, items)
        }
        Array::Character(values) => {
            let items: Vec<Option<String>> = values
                .iter()
                .map(|r| r.map(|c| c.map(|c| c.to_string())).map_err(|e| PyIOError::new_err(e.to_string())))
                .collect::<PyResult<_>>()?;
            PyList::new(py, items)
        }
        Array::String(values) => {
            let items: Vec<Option<String>> = values
                .iter()
                .map(|r| r.map(|s| s.map(|s| s.to_string())).map_err(|e| PyIOError::new_err(e.to_string())))
                .collect::<PyResult<_>>()?;
            PyList::new(py, items)
        }
    }
}
