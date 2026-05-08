use std::fs::File;
use std::io::{BufRead, BufReader, Read};

use noodles::bgzf;
use noodles::bgzf::io::Seek as BgzfSeek;
use pyo3::exceptions::PyIOError;
use pyo3::prelude::*;
use pyo3::types::PyBytes;

use super::virtual_position::PyVirtualPosition;

type Inner = bgzf::io::Reader<BufReader<File>>;

#[pyclass(name = "Reader", module = "ladle.bgzf")]
pub struct PyReader {
    inner: Option<Inner>,
}

impl PyReader {
    fn get(&mut self) -> PyResult<&mut Inner> {
        self.inner
            .as_mut()
            .ok_or_else(|| PyIOError::new_err("I/O operation on closed Reader"))
    }
}

#[pymethods]
impl PyReader {
    #[staticmethod]
    fn from_path(path: &str) -> PyResult<Self> {
        let file = File::open(path).map_err(|e| PyIOError::new_err(e.to_string()))?;
        let inner = bgzf::io::Reader::new(BufReader::new(file));
        Ok(Self { inner: Some(inner) })
    }

    fn read<'py>(&mut self, py: Python<'py>, size: usize) -> PyResult<Bound<'py, PyBytes>> {
        let reader = self.get()?;
        let mut buf = vec![0u8; size];
        let n = py
            .detach(|| reader.read(&mut buf))
            .map_err(|e| PyIOError::new_err(e.to_string()))?;
        buf.truncate(n);
        Ok(PyBytes::new(py, &buf))
    }

    fn read_all<'py>(&mut self, py: Python<'py>) -> PyResult<Bound<'py, PyBytes>> {
        let reader = self.get()?;
        let mut buf = Vec::new();
        py.detach(|| reader.read_to_end(&mut buf))
            .map_err(|e| PyIOError::new_err(e.to_string()))?;
        Ok(PyBytes::new(py, &buf))
    }

    fn read_line<'py>(&mut self, py: Python<'py>) -> PyResult<Bound<'py, PyBytes>> {
        let reader = self.get()?;
        let mut buf = Vec::new();
        py.detach(|| reader.read_until(b'\n', &mut buf))
            .map_err(|e| PyIOError::new_err(e.to_string()))?;
        Ok(PyBytes::new(py, &buf))
    }

    fn position(&mut self) -> PyResult<u64> {
        Ok(self.get()?.position())
    }

    fn virtual_position(&mut self) -> PyResult<PyVirtualPosition> {
        Ok(PyVirtualPosition::from(self.get()?.virtual_position()))
    }

    fn seek(&mut self, vpos: &PyVirtualPosition) -> PyResult<PyVirtualPosition> {
        let result = self
            .get()?
            .seek_to_virtual_position(vpos.inner)
            .map_err(|e| PyIOError::new_err(e.to_string()))?;
        Ok(PyVirtualPosition::from(result))
    }

    fn close(&mut self) {
        self.inner = None;
    }

    fn __enter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __exit__(
        &mut self,
        _exc_type: &Bound<'_, PyAny>,
        _exc_val: &Bound<'_, PyAny>,
        _exc_tb: &Bound<'_, PyAny>,
    ) {
        self.close();
    }

    fn __repr__(&self) -> &str {
        if self.inner.is_some() {
            "Reader(<open>)"
        } else {
            "Reader(<closed>)"
        }
    }
}
