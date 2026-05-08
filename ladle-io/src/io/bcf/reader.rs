use std::fs::File;
#[cfg(unix)]
use std::os::unix::io::{FromRawFd, RawFd};

use pyo3::exceptions::PyIOError;
use pyo3::prelude::*;

use crate::io::core::PyRegion;
use crate::io::vcf::header::PyHeader;
use super::batch::PyBcfRecordBatch;
use super::record::PyRecord;

type Inner = noodles::bcf::io::Reader<noodles::bgzf::io::Reader<File>>;
type InnerIndexedReader = noodles::bcf::io::IndexedReader<noodles::bgzf::io::Reader<File>>;

// ---------------------------------------------------------------------------
// Reader (sequential)
// ---------------------------------------------------------------------------

#[pyclass(name = "Reader", module = "ladle.bcf")]
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
        let inner = noodles::bcf::io::Reader::new(file);
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
            return Err(PyIOError::new_err(std::io::Error::last_os_error().to_string()));
        }
        let file = unsafe { File::from_raw_fd(owned_fd) };
        let inner = noodles::bcf::io::Reader::new(file);
        Ok(Self { inner: Some(inner) })
    }

    fn read_header(&mut self) -> PyResult<PyHeader> {
        self.get()?
            .read_header()
            .map(PyHeader::from)
            .map_err(|e| PyIOError::new_err(e.to_string()))
    }

    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(&mut self) -> PyResult<Option<PyRecord>> {
        let reader = self.get()?;
        let mut record = noodles::bcf::Record::default();
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

    fn records_to_batch(&mut self, py: Python<'_>, header: &PyHeader) -> PyResult<PyBcfRecordBatch> {
        let reader = self.get()?;
        let mut records: Vec<noodles::bcf::Record> = Vec::new();
        let mut record = noodles::bcf::Record::default();
        py.detach(|| -> PyResult<()> {
            loop {
                let n = reader
                    .read_record(&mut record)
                    .map_err(|e| PyIOError::new_err(e.to_string()))?;
                if n == 0 { break; }
                records.push(record.clone());
            }
            Ok(())
        })?;
        PyBcfRecordBatch::try_new_with_header(records, &header.inner)
            .map_err(|e| PyIOError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> &str {
        if self.inner.is_some() { "Reader(<open>)" } else { "Reader(<closed>)" }
    }
}

// ---------------------------------------------------------------------------
// IndexedReader (region-based queries)
// ---------------------------------------------------------------------------

// SAFETY: BinningIndex is not Send, but PyO3 holds the GIL on every method call,
// so PyIndexedReader is accessed from at most one thread at a time.
#[pyclass(name = "IndexedReader", module = "ladle.bcf")]
pub struct PyIndexedReader {
    inner: Option<InnerIndexedReader>,
}

unsafe impl Send for PyIndexedReader {}
unsafe impl Sync for PyIndexedReader {}

impl PyIndexedReader {
    fn get(&mut self) -> PyResult<&mut InnerIndexedReader> {
        self.inner
            .as_mut()
            .ok_or_else(|| PyIOError::new_err("I/O operation on closed IndexedReader"))
    }
}

#[pymethods]
impl PyIndexedReader {
    #[staticmethod]
    fn from_path(path: &str) -> PyResult<Self> {
        let inner = noodles::bcf::io::indexed_reader::Builder::default()
            .build_from_path(path)
            .map_err(|e| PyIOError::new_err(e.to_string()))?;
        Ok(Self { inner: Some(inner) })
    }

    fn read_header(&mut self) -> PyResult<PyHeader> {
        self.get()?
            .read_header()
            .map(PyHeader::from)
            .map_err(|e| PyIOError::new_err(e.to_string()))
    }

    fn query(&mut self, header: &PyHeader, region: &PyRegion) -> PyResult<PyQuery> {
        let reader = self.get()?;
        let mut query = reader
            .query(&header.inner, &region.inner)
            .map_err(|e| PyIOError::new_err(e.to_string()))?;

        let mut records = Vec::new();
        let mut record = noodles::bcf::Record::default();
        loop {
            let n = query
                .read_record(&mut record)
                .map_err(|e| PyIOError::new_err(e.to_string()))?;
            if n == 0 {
                break;
            }
            records.push(record.clone());
        }

        Ok(PyQuery { records, pos: 0 })
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
        if self.inner.is_some() { "IndexedReader(<open>)" } else { "IndexedReader(<closed>)" }
    }
}

// ---------------------------------------------------------------------------
// Query (eagerly collected region results)
// ---------------------------------------------------------------------------

#[pyclass(name = "Query", module = "ladle.bcf")]
pub struct PyQuery {
    records: Vec<noodles::bcf::Record>,
    pos: usize,
}

#[pymethods]
impl PyQuery {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(&mut self) -> Option<PyRecord> {
        if self.pos < self.records.len() {
            let rec = self.records[self.pos].clone();
            self.pos += 1;
            Some(PyRecord::from(rec))
        } else {
            None
        }
    }

    fn __len__(&self) -> usize {
        self.records.len()
    }

    fn __repr__(&self) -> String {
        format!("Query(<{} records>)", self.records.len())
    }
}
