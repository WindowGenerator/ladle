use std::fs::File;

use noodles::vcf::variant::io::Write as VcfWrite;
use pyo3::exceptions::PyIOError;
use pyo3::prelude::*;

use crate::io::vcf::header::PyHeader;
use crate::io::vcf::record::PyRecord as VcfPyRecord;
use super::record::PyRecord;

type Inner = noodles::bcf::io::Writer<noodles::bgzf::io::Writer<File>>;

#[pyclass(name = "Writer", module = "ladle.bcf")]
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
        Ok(Self { inner: Some(noodles::bcf::io::Writer::new(file)) })
    }

    fn write_header(&mut self, header: &PyHeader) -> PyResult<()> {
        self.get()?
            .write_header(&header.inner)
            .map_err(|e| PyIOError::new_err(e.to_string()))
    }

    fn write_record(&mut self, header: &PyHeader, record: &PyRecord) -> PyResult<()> {
        self.get()?
            .write_variant_record(&header.inner, &record.inner)
            .map_err(|e| PyIOError::new_err(e.to_string()))
    }

    fn write_vcf_record(&mut self, header: &PyHeader, record: &VcfPyRecord) -> PyResult<()> {
        self.get()?
            .write_variant_record(&header.inner, &record.inner)
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
