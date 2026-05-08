use std::fs::File;
use std::io::BufWriter;

use noodles::vcf::variant::io::Write as VcfWrite;
use pyo3::exceptions::PyIOError;
use pyo3::prelude::*;

use super::header::PyHeader;
use super::record::PyRecord;

type Inner = noodles::vcf::io::Writer<BufWriter<File>>;

#[pyclass(name = "Writer", module = "ladle.vcf")]
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
        Ok(Self { inner: Some(noodles::vcf::io::Writer::new(BufWriter::new(file))) })
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
        if self.inner.is_some() { "Writer(<open>)" } else { "Writer(<closed>)" }
    }
}
