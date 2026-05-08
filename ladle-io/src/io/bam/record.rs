use bstr::ByteSlice;
use noodles::sam::alignment::record::data::field::Value;
use noodles::sam::alignment::record::data::field::value::Array;
use pyo3::exceptions::PyIOError;
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict, PyList};

use crate::io::core::PyPosition;
use crate::io::sam::flags::PyFlags;
use crate::io::sam::mapping_quality::PyMappingQuality;

#[pyclass(name = "Record", module = "ladle.bam", from_py_object)]
#[derive(Clone)]
pub struct PyRecord {
    pub inner: noodles::bam::Record,
}

impl From<noodles::bam::Record> for PyRecord {
    fn from(inner: noodles::bam::Record) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyRecord {
    fn name<'py>(&self, py: Python<'py>) -> Option<pyo3::Bound<'py, PyBytes>> {
        self.inner.name().map(|n| PyBytes::new(py, n.as_bytes()))
    }

    fn flags(&self) -> PyResult<PyFlags> {
        Ok(PyFlags::from(self.inner.flags()))
    }

    fn reference_sequence_id(&self) -> PyResult<Option<usize>> {
        match self.inner.reference_sequence_id() {
            None => Ok(None),
            Some(Ok(id)) => Ok(Some(id)),
            Some(Err(e)) => Err(PyIOError::new_err(e.to_string())),
        }
    }

    fn alignment_start(&self) -> PyResult<Option<PyPosition>> {
        match self.inner.alignment_start() {
            None => Ok(None),
            Some(Ok(pos)) => Ok(Some(PyPosition::from(pos))),
            Some(Err(e)) => Err(PyIOError::new_err(e.to_string())),
        }
    }

    fn mapping_quality(&self) -> PyResult<Option<PyMappingQuality>> {
        Ok(self.inner.mapping_quality().map(PyMappingQuality::from))
    }

    fn cigar<'py>(&self, py: Python<'py>) -> pyo3::Bound<'py, PyBytes> {
        PyBytes::new(py, self.inner.cigar().as_ref())
    }

    fn mate_reference_sequence_id(&self) -> PyResult<Option<usize>> {
        match self.inner.mate_reference_sequence_id() {
            None => Ok(None),
            Some(Ok(id)) => Ok(Some(id)),
            Some(Err(e)) => Err(PyIOError::new_err(e.to_string())),
        }
    }

    fn mate_alignment_start(&self) -> PyResult<Option<PyPosition>> {
        match self.inner.mate_alignment_start() {
            None => Ok(None),
            Some(Ok(pos)) => Ok(Some(PyPosition::from(pos))),
            Some(Err(e)) => Err(PyIOError::new_err(e.to_string())),
        }
    }

    fn template_length(&self) -> i32 {
        self.inner.template_length()
    }

    fn sequence<'py>(&self, py: Python<'py>) -> pyo3::Bound<'py, PyBytes> {
        PyBytes::new(py, self.inner.sequence().as_ref())
    }

    fn quality_scores<'py>(&self, py: Python<'py>) -> pyo3::Bound<'py, PyBytes> {
        PyBytes::new(py, self.inner.quality_scores().as_ref())
    }

    fn data<'py>(&self, py: Python<'py>) -> PyResult<pyo3::Bound<'py, PyDict>> {
        let dict = PyDict::new(py);
        for result in self.inner.data().iter() {
            let (tag, value) = result.map_err(|e| PyIOError::new_err(e.to_string()))?;
            let key = PyBytes::new(py, tag.as_ref());
            let val = value_to_py(py, value)?;
            dict.set_item(key, val)?;
        }
        Ok(dict)
    }

    fn __repr__(&self) -> String {
        let name = self
            .inner
            .name()
            .and_then(|n| std::str::from_utf8(n.as_bytes()).ok())
            .unwrap_or("*");
        format!("Record(name={name:?})")
    }
}

fn value_to_py<'py>(py: Python<'py>, value: Value<'_>) -> PyResult<pyo3::Bound<'py, PyAny>> {
    use Value::*;
    Ok(match value {
        Character(c) => PyBytes::new(py, &[c]).into_any(),
        Int8(v) => v.into_pyobject(py)?.into_any(),
        UInt8(v) => v.into_pyobject(py)?.into_any(),
        Int16(v) => v.into_pyobject(py)?.into_any(),
        UInt16(v) => v.into_pyobject(py)?.into_any(),
        Int32(v) => v.into_pyobject(py)?.into_any(),
        UInt32(v) => v.into_pyobject(py)?.into_any(),
        Float(v) => v.into_pyobject(py)?.into_any(),
        String(s) => PyBytes::new(py, s.as_bytes()).into_any(),
        Hex(s) => PyBytes::new(py, s.as_bytes()).into_any(),
        Array(arr) => array_to_py(py, arr)?.into_any(),
    })
}

fn array_to_py<'py>(py: Python<'py>, arr: Array<'_>) -> PyResult<pyo3::Bound<'py, PyList>> {
    use Array::*;
    match arr {
        Int8(values) => {
            let items: Vec<i8> = values
                .iter()
                .map(|r| r.map_err(|e| PyIOError::new_err(e.to_string())))
                .collect::<PyResult<_>>()?;
            PyList::new(py, items)
        }
        UInt8(values) => {
            let items: Vec<u8> = values
                .iter()
                .map(|r| r.map_err(|e| PyIOError::new_err(e.to_string())))
                .collect::<PyResult<_>>()?;
            PyList::new(py, items)
        }
        Int16(values) => {
            let items: Vec<i16> = values
                .iter()
                .map(|r| r.map_err(|e| PyIOError::new_err(e.to_string())))
                .collect::<PyResult<_>>()?;
            PyList::new(py, items)
        }
        UInt16(values) => {
            let items: Vec<u16> = values
                .iter()
                .map(|r| r.map_err(|e| PyIOError::new_err(e.to_string())))
                .collect::<PyResult<_>>()?;
            PyList::new(py, items)
        }
        Int32(values) => {
            let items: Vec<i32> = values
                .iter()
                .map(|r| r.map_err(|e| PyIOError::new_err(e.to_string())))
                .collect::<PyResult<_>>()?;
            PyList::new(py, items)
        }
        UInt32(values) => {
            let items: Vec<u32> = values
                .iter()
                .map(|r| r.map_err(|e| PyIOError::new_err(e.to_string())))
                .collect::<PyResult<_>>()?;
            PyList::new(py, items)
        }
        Float(values) => {
            let items: Vec<f32> = values
                .iter()
                .map(|r| r.map_err(|e| PyIOError::new_err(e.to_string())))
                .collect::<PyResult<_>>()?;
            PyList::new(py, items)
        }
    }
}
