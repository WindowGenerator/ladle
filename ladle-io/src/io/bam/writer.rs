use std::fs::File;
use std::io::BufWriter;

use pyo3::exceptions::PyIOError;
use pyo3::prelude::*;

use noodles::sam::alignment::io::Write as _;

use crate::io::sam::header::PyHeader;
use crate::io::sam::record::PyRecord as SamPyRecord;
use super::record::PyRecord;

type Inner = noodles::bam::io::Writer<noodles::bgzf::io::Writer<BufWriter<File>>>;

#[pyclass(name = "Writer", module = "ladle.bam")]
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
    fn from_path(path: &str) -> PyResult<Self> {
        let file = File::create(path).map_err(|e| PyIOError::new_err(e.to_string()))?;
        let inner = noodles::bam::io::Writer::new(BufWriter::new(file));
        Ok(Self { inner: Some(inner) })
    }

    fn write_header(&mut self, header: &PyHeader) -> PyResult<()> {
        self.get()?
            .write_header(&header.inner)
            .map_err(|e| PyIOError::new_err(e.to_string()))
    }

    fn write_record(&mut self, header: &PyHeader, record: &PyRecord) -> PyResult<()> {
        self.get()?
            .write_record(&header.inner, &record.inner)
            .map_err(|e| PyIOError::new_err(e.to_string()))
    }

    fn write_sam_record(&mut self, header: &PyHeader, record: &SamPyRecord) -> PyResult<()> {
        self.get()?
            .write_alignment_record(&header.inner, &record.inner)
            .map_err(|e| PyIOError::new_err(e.to_string()))
    }

    fn close(&mut self) -> PyResult<()> {
        if let Some(mut w) = self.inner.take() {
            w.try_finish().map_err(|e| PyIOError::new_err(e.to_string()))?;
        }
        Ok(())
    }

    fn __enter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __exit__(
        &mut self,
        _exc_type: &Bound<'_, PyAny>,
        _exc_val: &Bound<'_, PyAny>,
        _exc_tb: &Bound<'_, PyAny>,
    ) -> PyResult<()> {
        self.close()
    }

    fn __repr__(&self) -> &str {
        if self.inner.is_some() { "Writer(<open>)" } else { "Writer(<closed>)" }
    }
}
