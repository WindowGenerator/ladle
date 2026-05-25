use pyo3::prelude::*;
use pyo3::types::PyBytes;

/// A single FASTQ record with name, sequence, and quality scores.
#[pyclass(name = "Record", module = "ladle.fastq", from_py_object)]
#[derive(Clone)]
pub struct PyRecord {
    pub inner: noodles::fastq::Record,
}

impl From<noodles::fastq::Record> for PyRecord {
    fn from(inner: noodles::fastq::Record) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyRecord {
    fn name<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, self.inner.name())
    }

    fn description<'py>(&self, py: Python<'py>) -> Option<Bound<'py, PyBytes>> {
        let d = self.inner.description();
        if d.is_empty() {
            None
        } else {
            Some(PyBytes::new(py, d))
        }
    }

    fn sequence<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, self.inner.sequence())
    }

    fn quality_scores<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, self.inner.quality_scores())
    }

    fn __repr__(&self) -> String {
        let name = std::str::from_utf8(self.inner.name()).unwrap_or("?");
        format!("Record(name={name:?})")
    }
}
