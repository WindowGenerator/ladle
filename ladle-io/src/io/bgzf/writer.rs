use std::fs::File;
use std::io::Write;

use noodles::bgzf;
use noodles::bgzf::io::writer::CompressionLevel;
use pyo3::exceptions::{PyIOError, PyValueError};
use pyo3::prelude::*;

use super::virtual_position::PyVirtualPosition;

// bgzf::Writer does its own block-buffering; no BufWriter needed.
type Inner = bgzf::io::Writer<File>;

#[pyclass(name = "Writer", module = "ladle.bgzf")]
pub struct PyWriter {
    inner: Option<Inner>,
}

impl PyWriter {
    fn get(&mut self) -> PyResult<&mut Inner> {
        self.inner
            .as_mut()
            .ok_or_else(|| PyIOError::new_err("I/O operation on closed Writer"))
    }
}

#[pymethods]
impl PyWriter {
    #[staticmethod]
    #[pyo3(signature = (path, compression_level=6))]
    fn from_path(path: &str, compression_level: u8) -> PyResult<Self> {
        let level = CompressionLevel::new(compression_level).ok_or_else(|| {
            PyValueError::new_err(format!("invalid compression level: {compression_level}"))
        })?;
        let file = File::create(path).map_err(|e| PyIOError::new_err(e.to_string()))?;
        let inner = bgzf::io::writer::Builder::default()
            .set_compression_level(level)
            .build_from_writer(file);
        Ok(Self { inner: Some(inner) })
    }

    fn write(&mut self, py: Python<'_>, data: &[u8]) -> PyResult<usize> {
        let len = data.len();
        let writer = self.get()?;
        // Use write_all so callers don't need to loop; matches Python's write() contract.
        py.detach(|| writer.write_all(data))
            .map_err(|e| PyIOError::new_err(e.to_string()))?;
        Ok(len)
    }

    fn flush(&mut self, py: Python<'_>) -> PyResult<()> {
        let writer = self.get()?;
        py.detach(|| writer.flush())
            .map_err(|e| PyIOError::new_err(e.to_string()))
    }

    fn finish(&mut self, py: Python<'_>) -> PyResult<()> {
        let writer = self
            .inner
            .as_mut()
            .ok_or_else(|| PyIOError::new_err("I/O operation on closed Writer"))?;
        py.detach(|| writer.try_finish())
            .map_err(|e| PyIOError::new_err(e.to_string()))?;
        self.inner = None;
        Ok(())
    }

    fn position(&mut self) -> PyResult<u64> {
        Ok(self.get()?.position())
    }

    fn virtual_position(&mut self) -> PyResult<PyVirtualPosition> {
        Ok(PyVirtualPosition::from(self.get()?.virtual_position()))
    }

    fn close(&mut self, py: Python<'_>) -> PyResult<()> {
        if self.inner.is_some() {
            self.finish(py)?;
        }
        Ok(())
    }

    fn __enter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __exit__(
        &mut self,
        py: Python<'_>,
        _exc_type: &Bound<'_, PyAny>,
        _exc_val: &Bound<'_, PyAny>,
        _exc_tb: &Bound<'_, PyAny>,
    ) -> PyResult<()> {
        self.close(py)
    }

    fn __repr__(&self) -> &str {
        if self.inner.is_some() {
            "Writer(<open>)"
        } else {
            "Writer(<closed>)"
        }
    }
}
