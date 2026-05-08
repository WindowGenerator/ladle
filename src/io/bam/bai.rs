use pyo3::exceptions::PyIOError;
use pyo3::prelude::*;

#[pyclass(name = "Index", module = "ladle.bam.bai")]
pub struct PyBaiIndex {
    pub inner: noodles::bam::bai::Index,
}

#[pymethods]
impl PyBaiIndex {
    #[staticmethod]
    fn read_from_path(path: &str) -> PyResult<Self> {
        noodles::bam::bai::fs::read(path)
            .map(|inner| Self { inner })
            .map_err(|e| PyIOError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> &str {
        "Index(<bai>)"
    }
}

#[pymodule(submodule)]
pub mod bai {
    #[pymodule_export]
    use super::PyBaiIndex as Index;
}
