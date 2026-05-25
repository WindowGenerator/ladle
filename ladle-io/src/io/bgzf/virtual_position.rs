use noodles::bgzf::VirtualPosition;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

/// A BGZF virtual position encoding both block offset and within-block offset.
///
/// Virtual positions are used with [`Reader.seek`] for random access within a BGZF file.
///
/// Examples
/// --------
/// >>> vpos = bgzf.VirtualPosition(block_offset, within_offset)
/// >>> reader.seek(vpos)
#[pyclass(
    name = "VirtualPosition",
    module = "ladle.bgzf",
    frozen,
    from_py_object
)]
#[derive(Clone)]
pub struct PyVirtualPosition {
    pub inner: VirtualPosition,
}

impl From<VirtualPosition> for PyVirtualPosition {
    fn from(inner: VirtualPosition) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyVirtualPosition {
    #[new]
    fn new(compressed: u64, uncompressed: u16) -> PyResult<Self> {
        VirtualPosition::new(compressed, uncompressed)
            .map(|vp| Self { inner: vp })
            .ok_or_else(|| {
                PyValueError::new_err(format!(
                    "compressed position {compressed} exceeds maximum (2^48 - 1)"
                ))
            })
    }

    #[classattr]
    #[allow(non_snake_case)]
    fn MIN() -> Self {
        Self {
            inner: VirtualPosition::MIN,
        }
    }

    #[classattr]
    #[allow(non_snake_case)]
    fn MAX() -> Self {
        Self {
            inner: VirtualPosition::MAX,
        }
    }

    #[staticmethod]
    fn from_u64(n: u64) -> Self {
        Self {
            inner: VirtualPosition::from(n),
        }
    }

    fn compressed(&self) -> u64 {
        self.inner.compressed()
    }

    fn uncompressed(&self) -> u16 {
        self.inner.uncompressed()
    }

    fn __int__(&self) -> u64 {
        u64::from(self.inner)
    }

    fn __str__(&self) -> String {
        format!("{}:{}", self.inner.compressed(), self.inner.uncompressed())
    }

    fn __repr__(&self) -> String {
        format!(
            "VirtualPosition(compressed={}, uncompressed={})",
            self.inner.compressed(),
            self.inner.uncompressed()
        )
    }

    fn __hash__(&self) -> u64 {
        u64::from(self.inner)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner == other.inner
    }

    fn __lt__(&self, other: &Self) -> bool {
        self.inner < other.inner
    }

    fn __le__(&self, other: &Self) -> bool {
        self.inner <= other.inner
    }

    fn __gt__(&self, other: &Self) -> bool {
        self.inner > other.inner
    }

    fn __ge__(&self, other: &Self) -> bool {
        self.inner >= other.inner
    }
}
