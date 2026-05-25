use noodles::sam::alignment::record::MappingQuality;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

/// Mapping quality score (MAPQ) in the range 0–254. Value 255 means unavailable.
///
/// Examples
/// --------
/// >>> mq = sam.MappingQuality(60)
/// >>> mq.get()
/// 60
#[pyclass(name = "MappingQuality", module = "ladle.sam", frozen, from_py_object)]
#[derive(Clone)]
pub struct PyMappingQuality {
    pub inner: MappingQuality,
}

impl From<MappingQuality> for PyMappingQuality {
    fn from(inner: MappingQuality) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyMappingQuality {
    #[new]
    fn new(n: u8) -> PyResult<Self> {
        MappingQuality::new(n)
            .map(|mq| Self { inner: mq })
            .ok_or_else(|| PyValueError::new_err("mapping quality 255 means missing/unavailable"))
    }

    #[classattr]
    #[allow(non_snake_case)]
    fn MIN() -> Self {
        Self {
            inner: MappingQuality::MIN,
        }
    }

    #[classattr]
    #[allow(non_snake_case)]
    fn MAX() -> Self {
        Self {
            inner: MappingQuality::MAX,
        }
    }

    fn get(&self) -> u8 {
        self.inner.get()
    }

    fn __int__(&self) -> u8 {
        self.inner.get()
    }

    fn __str__(&self) -> String {
        self.inner.get().to_string()
    }

    fn __repr__(&self) -> String {
        format!("MappingQuality({})", self.inner.get())
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner == other.inner
    }

    fn __lt__(&self, other: &Self) -> bool {
        self.inner.get() < other.inner.get()
    }

    fn __le__(&self, other: &Self) -> bool {
        self.inner.get() <= other.inner.get()
    }

    fn __gt__(&self, other: &Self) -> bool {
        self.inner.get() > other.inner.get()
    }

    fn __ge__(&self, other: &Self) -> bool {
        self.inner.get() >= other.inner.get()
    }

    fn __hash__(&self) -> u8 {
        self.inner.get()
    }
}
