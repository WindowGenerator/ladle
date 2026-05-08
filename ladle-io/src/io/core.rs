use std::ops::Bound as StdBound;

use bstr::ByteSlice;
use noodles::core::region::Interval;
use noodles::core::{Position, Region};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyBytes;

// ---------------------------------------------------------------------------
// Position
// ---------------------------------------------------------------------------

#[pyclass(name = "Position", module = "ladle.core", frozen, from_py_object)]
#[derive(Clone)]
pub struct PyPosition {
    inner: Position,
}

impl From<Position> for PyPosition {
    fn from(inner: Position) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyPosition {
    #[new]
    fn new(n: usize) -> PyResult<Self> {
        Position::new(n)
            .map(|p| Self { inner: p })
            .ok_or_else(|| PyValueError::new_err("position must be >= 1"))
    }

    #[classattr]
    #[allow(non_snake_case)]
    fn MIN() -> Self {
        Self {
            inner: Position::MIN,
        }
    }

    #[classattr]
    #[allow(non_snake_case)]
    fn MAX() -> Self {
        Self {
            inner: Position::MAX,
        }
    }

    fn get(&self) -> usize {
        self.inner.get()
    }

    fn checked_add(&self, n: usize) -> Option<Self> {
        self.inner.checked_add(n).map(|p| Self { inner: p })
    }

    #[staticmethod]
    fn parse(s: &str) -> PyResult<Self> {
        s.parse::<Position>()
            .map(|p| Self { inner: p })
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    fn __int__(&self) -> usize {
        self.inner.get()
    }

    fn __str__(&self) -> String {
        self.inner.to_string()
    }

    fn __repr__(&self) -> String {
        format!("Position({})", self.inner.get())
    }

    fn __hash__(&self) -> usize {
        self.inner.get()
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

// ---------------------------------------------------------------------------
// Interval
// ---------------------------------------------------------------------------

#[pyclass(name = "Interval", module = "ladle.core", frozen, from_py_object)]
#[derive(Clone)]
pub struct PyInterval {
    inner: Interval,
}

impl From<Interval> for PyInterval {
    fn from(inner: Interval) -> Self {
        Self { inner }
    }
}

fn pos_from_opt(n: Option<usize>) -> PyResult<Option<Position>> {
    match n {
        None => Ok(None),
        Some(v) => Position::new(v)
            .ok_or_else(|| PyValueError::new_err("position must be >= 1"))
            .map(Some),
    }
}

#[pymethods]
impl PyInterval {
    #[new]
    #[pyo3(signature = (start=None, end=None))]
    fn new(start: Option<usize>, end: Option<usize>) -> PyResult<Self> {
        let start = pos_from_opt(start)?;
        let end = pos_from_opt(end)?;
        let interval = match (start, end) {
            (Some(s), Some(e)) => Interval::from(s..=e),
            (Some(s), None) => Interval::from(s..),
            (None, Some(e)) => Interval::from(..=e),
            (None, None) => Interval::from(..),
        };
        Ok(Self { inner: interval })
    }

    fn start(&self) -> Option<PyPosition> {
        self.inner.start().map(PyPosition::from)
    }

    fn end(&self) -> Option<PyPosition> {
        self.inner.end().map(PyPosition::from)
    }

    fn contains(&self, pos: &PyPosition) -> bool {
        self.inner.contains(pos.inner)
    }

    fn intersects(&self, other: &PyInterval) -> bool {
        self.inner.intersects(other.inner)
    }

    #[staticmethod]
    fn parse(s: &str) -> PyResult<Self> {
        s.parse::<Interval>()
            .map(|i| Self { inner: i })
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    fn __str__(&self) -> String {
        self.inner.to_string()
    }

    fn __repr__(&self) -> String {
        let s = match self.inner.start() {
            Some(p) => format!("start={}", p.get()),
            None => "start=None".into(),
        };
        let e = match self.inner.end() {
            Some(p) => format!("end={}", p.get()),
            None => "end=None".into(),
        };
        format!("Interval({s}, {e})")
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner == other.inner
    }

    fn __hash__(&self) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut h = DefaultHasher::new();
        self.inner.start().map(|p| p.get()).hash(&mut h);
        self.inner.end().map(|p| p.get()).hash(&mut h);
        h.finish()
    }
}

// ---------------------------------------------------------------------------
// Region
// ---------------------------------------------------------------------------

#[pyclass(name = "Region", module = "ladle.core", frozen, from_py_object)]
#[derive(Clone)]
pub struct PyRegion {
    pub inner: Region,
}

fn bound_to_opt(b: StdBound<Position>) -> Option<PyPosition> {
    match b {
        StdBound::Included(p) => Some(PyPosition::from(p)),
        _ => None,
    }
}

#[pymethods]
impl PyRegion {
    #[new]
    fn new(name: &pyo3::Bound<'_, PyAny>, interval: &PyInterval) -> PyResult<Self> {
        let name_bytes: Vec<u8> = if let Ok(s) = name.extract::<&str>() {
            s.as_bytes().to_vec()
        } else if let Ok(b) = name.extract::<&[u8]>() {
            b.to_vec()
        } else {
            return Err(PyValueError::new_err("name must be str or bytes"));
        };
        let region = Region::new(name_bytes, interval.inner);
        Ok(Self { inner: region })
    }

    fn name<'py>(&self, py: Python<'py>) -> pyo3::Bound<'py, PyBytes> {
        PyBytes::new(py, self.inner.name().as_bytes())
    }

    fn start(&self) -> Option<PyPosition> {
        bound_to_opt(self.inner.start())
    }

    fn end(&self) -> Option<PyPosition> {
        bound_to_opt(self.inner.end())
    }

    fn interval(&self) -> PyInterval {
        PyInterval::from(self.inner.interval())
    }

    #[staticmethod]
    fn parse(s: &str) -> PyResult<Self> {
        s.parse::<Region>()
            .map(|r| Self { inner: r })
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    fn __str__(&self) -> String {
        self.inner.to_string()
    }

    fn __repr__(&self) -> String {
        let name = std::str::from_utf8(self.inner.name().as_bytes()).unwrap_or("?");
        format!("Region({name:?}, {})", self.inner.interval())
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

// ---------------------------------------------------------------------------
// Submodule
// ---------------------------------------------------------------------------

#[pymodule(submodule)]
pub mod core {
    #[pymodule_export]
    use super::PyInterval as Interval;
    #[pymodule_export]
    use super::PyPosition as Position;
    #[pymodule_export]
    use super::PyRegion as Region;
}
