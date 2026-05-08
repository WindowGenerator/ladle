use std::fs::File;
use std::io::BufWriter;

use pyo3::exceptions::PyIOError;
use pyo3::prelude::*;

use super::record::PyRecord;

type Inner = noodles::fastq::io::Writer<BufWriter<File>>;

#[pyclass(name = "Writer", module = "ladle.fastq")]
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
        let inner = noodles::fastq::io::Writer::new(BufWriter::new(file));
        Ok(Self { inner: Some(inner) })
    }

    fn write_record(&mut self, rec: &PyRecord) -> PyResult<()> {
        self.get()?
            .write_record(&rec.inner)
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
