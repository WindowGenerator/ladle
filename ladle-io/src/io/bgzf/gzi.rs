use noodles::bgzf::gzi as noodles_gzi;
use pyo3::exceptions::PyIOError;
use pyo3::prelude::*;

use super::virtual_position::PyVirtualPosition;

#[pyclass(name = "Index", module = "ladle.bgzf.gzi", from_py_object)]
#[derive(Clone)]
pub struct PyIndex {
    inner: noodles_gzi::Index,
}

#[pymethods]
impl PyIndex {
    #[staticmethod]
    fn from_path(path: &str) -> PyResult<Self> {
        noodles_gzi::fs::read(path)
            .map(|idx| Self { inner: idx })
            .map_err(|e| PyIOError::new_err(e.to_string()))
    }

    fn query(&self, pos: u64) -> PyResult<PyVirtualPosition> {
        self.inner
            .query(pos)
            .map(PyVirtualPosition::from)
            .map_err(|e| PyIOError::new_err(e.to_string()))
    }

    fn __len__(&self) -> usize {
        self.inner.as_ref().len()
    }

    fn __repr__(&self) -> String {
        format!("gzi.Index(<{} entries>)", self.inner.as_ref().len())
    }
}

#[pymodule(submodule)]
pub mod gzi {
    #[pymodule_export]
    use super::PyIndex as Index;
}
