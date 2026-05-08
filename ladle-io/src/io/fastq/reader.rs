use std::fs::File;
use std::io::BufReader;
#[cfg(unix)]
use std::os::unix::io::{FromRawFd, RawFd};

use pyo3::exceptions::PyIOError;
use pyo3::prelude::*;

use super::record::PyRecord;

type Inner = noodles::fastq::io::Reader<BufReader<File>>;

#[pyclass(name = "Reader", module = "ladle.fastq")]
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
        let inner = noodles::fastq::io::Reader::new(BufReader::new(file));
        Ok(Self { inner: Some(inner) })
    }

    #[cfg(unix)]
    #[staticmethod]
    fn from_fd(obj: &Bound<'_, PyAny>) -> PyResult<Self> {
        let fd: RawFd = obj
            .call_method0("fileno")
            .map_err(|_| PyIOError::new_err("fd object must have a fileno() method"))?
            .extract()?;
        let owned_fd = unsafe { libc::dup(fd) };
        if owned_fd < 0 {
            return Err(PyIOError::new_err(
                std::io::Error::last_os_error().to_string(),
            ));
        }
        let file = unsafe { File::from_raw_fd(owned_fd) };
        let inner = noodles::fastq::io::Reader::new(BufReader::new(file));
        Ok(Self { inner: Some(inner) })
    }

    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(&mut self) -> PyResult<Option<PyRecord>> {
        let reader = self.get()?;
        let mut record = noodles::fastq::Record::default();
        let n = reader
            .read_record(&mut record)
            .map_err(|e| PyIOError::new_err(e.to_string()))?;
        if n == 0 {
            Ok(None)
        } else {
            Ok(Some(PyRecord::from(record)))
        }
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
