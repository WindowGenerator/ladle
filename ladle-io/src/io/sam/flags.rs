use noodles::sam::alignment::record::Flags;
use pyo3::prelude::*;

#[pyclass(name = "Flags", module = "ladle.sam", frozen, from_py_object)]
#[derive(Clone)]
pub struct PyFlags {
    pub inner: Flags,
}

impl From<Flags> for PyFlags {
    fn from(inner: Flags) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyFlags {
    // Class-level bit constants
    #[classattr]
    #[allow(non_snake_case)]
    fn SEGMENTED() -> u16 {
        0x0001
    }
    #[classattr]
    #[allow(non_snake_case)]
    fn PROPERLY_SEGMENTED() -> u16 {
        0x0002
    }
    #[classattr]
    #[allow(non_snake_case)]
    fn UNMAPPED() -> u16 {
        0x0004
    }
    #[classattr]
    #[allow(non_snake_case)]
    fn MATE_UNMAPPED() -> u16 {
        0x0008
    }
    #[classattr]
    #[allow(non_snake_case)]
    fn REVERSE_COMPLEMENTED() -> u16 {
        0x0010
    }
    #[classattr]
    #[allow(non_snake_case)]
    fn MATE_REVERSE_COMPLEMENTED() -> u16 {
        0x0020
    }
    #[classattr]
    #[allow(non_snake_case)]
    fn FIRST_SEGMENT() -> u16 {
        0x0040
    }
    #[classattr]
    #[allow(non_snake_case)]
    fn LAST_SEGMENT() -> u16 {
        0x0080
    }
    #[classattr]
    #[allow(non_snake_case)]
    fn SECONDARY() -> u16 {
        0x0100
    }
    #[classattr]
    #[allow(non_snake_case)]
    fn QC_FAIL() -> u16 {
        0x0200
    }
    #[classattr]
    #[allow(non_snake_case)]
    fn DUPLICATE() -> u16 {
        0x0400
    }
    #[classattr]
    #[allow(non_snake_case)]
    fn SUPPLEMENTARY() -> u16 {
        0x0800
    }

    #[new]
    fn new(bits: u16) -> Self {
        Self {
            inner: Flags::from(bits),
        }
    }

    fn bits(&self) -> u16 {
        u16::from(self.inner)
    }

    fn __int__(&self) -> u16 {
        u16::from(self.inner)
    }

    fn __repr__(&self) -> String {
        format!("Flags(0x{:04x})", u16::from(self.inner))
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner == other.inner
    }

    fn __hash__(&self) -> u16 {
        u16::from(self.inner)
    }

    fn __and__(&self, other: &Self) -> Self {
        Self::from(Flags::from(u16::from(self.inner) & u16::from(other.inner)))
    }

    fn __or__(&self, other: &Self) -> Self {
        Self::from(Flags::from(u16::from(self.inner) | u16::from(other.inner)))
    }

    fn is_segmented(&self) -> bool {
        self.inner.is_segmented()
    }
    fn is_properly_segmented(&self) -> bool {
        self.inner.is_properly_segmented()
    }
    fn is_unmapped(&self) -> bool {
        self.inner.is_unmapped()
    }
    fn is_mate_unmapped(&self) -> bool {
        self.inner.is_mate_unmapped()
    }
    fn is_reverse_complemented(&self) -> bool {
        self.inner.is_reverse_complemented()
    }
    fn is_mate_reverse_complemented(&self) -> bool {
        self.inner.is_mate_reverse_complemented()
    }
    fn is_first_segment(&self) -> bool {
        self.inner.is_first_segment()
    }
    fn is_last_segment(&self) -> bool {
        self.inner.is_last_segment()
    }
    fn is_secondary(&self) -> bool {
        self.inner.is_secondary()
    }
    fn is_qc_fail(&self) -> bool {
        self.inner.is_qc_fail()
    }
    fn is_duplicate(&self) -> bool {
        self.inner.is_duplicate()
    }
    fn is_supplementary(&self) -> bool {
        self.inner.is_supplementary()
    }
}
