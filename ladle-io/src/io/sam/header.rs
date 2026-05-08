use bstr::ByteSlice;
use noodles::sam::Header;
use noodles::sam::io::Writer as SamWriter;
use pyo3::exceptions::{PyIOError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict};

#[pyclass(name = "Header", module = "ladle.sam", from_py_object)]
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

    fn reference_sequences<'py>(&self, py: Python<'py>) -> PyResult<pyo3::Bound<'py, PyDict>> {
        let dict = PyDict::new(py);
        for (name, map) in self.inner.reference_sequences() {
            let key = PyBytes::new(py, name.as_bytes());
            let len: usize = map.length().get();
            dict.set_item(key, len)?;
        }
        Ok(dict)
    }

    fn __str__(&self) -> PyResult<String> {
        let mut buf = Vec::new();
        let mut w = SamWriter::new(&mut buf);
        w.write_header(&self.inner)
            .map_err(|e| PyIOError::new_err(e.to_string()))?;
        String::from_utf8(buf).map_err(|e| PyIOError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "Header(<{} reference sequences>)",
            self.inner.reference_sequences().len()
        )
    }
}
