use noodles::vcf::Header;
use noodles::vcf::header::record::value::map::format::Type as FormatType;
use noodles::vcf::header::record::value::map::info::Type as InfoType;
use pyo3::exceptions::{PyIOError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyString, PyTuple};

#[pyclass(name = "Header", module = "ladle.vcf", from_py_object)]
#[derive(Clone)]
pub struct PyHeader {
    pub inner: Header,
}

impl From<Header> for PyHeader {
    fn from(inner: Header) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyHeader {
    #[new]
    fn new() -> Self {
        Self {
            inner: Header::default(),
        }
    }

    #[staticmethod]
    fn parse(s: &str) -> PyResult<Self> {
        s.parse::<Header>()
            .map(|h| Self { inner: h })
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    fn sample_names<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyList>> {
        let names: Vec<Bound<'_, PyString>> = self
            .inner
            .sample_names()
            .iter()
            .map(|n| PyString::new(py, n))
            .collect();
        PyList::new(py, names)
    }

    fn contigs<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let dict = PyDict::new(py);
        for (name, map) in self.inner.contigs() {
            let length: Option<usize> = map.length();
            dict.set_item(name, length)?;
        }
        Ok(dict)
    }

    fn __str__(&self) -> PyResult<String> {
        let mut buf = Vec::new();
        {
            let mut w = noodles::vcf::io::Writer::new(&mut buf);
            w.write_header(&self.inner)
                .map_err(|e| PyIOError::new_err(e.to_string()))?;
        }
        String::from_utf8(buf).map_err(|e| PyIOError::new_err(e.to_string()))
    }

    fn info_fields<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyList>> {
        let items: Vec<Bound<'_, PyTuple>> = self
            .inner
            .infos()
            .iter()
            .map(|(key, map)| {
                let type_str = match map.ty() {
                    InfoType::Integer => "Integer",
                    InfoType::Float => "Float",
                    InfoType::Flag => "Flag",
                    InfoType::Character => "Character",
                    InfoType::String => "String",
                };
                PyTuple::new(py, [key.as_str(), type_str])
            })
            .collect::<Result<_, _>>()?;
        PyList::new(py, items)
    }

    fn format_fields<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyList>> {
        let items: Vec<Bound<'_, PyTuple>> = self
            .inner
            .formats()
            .iter()
            .map(|(key, map)| {
                let type_str = match map.ty() {
                    FormatType::Integer => "Integer",
                    FormatType::Float => "Float",
                    FormatType::Character => "Character",
                    FormatType::String => "String",
                };
                PyTuple::new(py, [key.as_str(), type_str])
            })
            .collect::<Result<_, _>>()?;
        PyList::new(py, items)
    }

    fn __repr__(&self) -> String {
        format!(
            "Header(<{} samples, {} contigs>)",
            self.inner.sample_names().len(),
            self.inner.contigs().len(),
        )
    }
}
