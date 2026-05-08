use pyo3::prelude::*;

pub mod arrow_utils;
mod intervals;

use intervals::ops::{py_count_overlaps, py_nearest, py_overlap};

#[pymodule]
mod _ladle_ops {
    use pyo3::prelude::*;

    use super::{py_count_overlaps, py_nearest, py_overlap};

    #[pymodule_init]
    fn init(m: &Bound<'_, PyModule>) -> PyResult<()> {
        m.add_function(wrap_pyfunction!(py_overlap, m)?)?;
        m.add_function(wrap_pyfunction!(py_nearest, m)?)?;
        m.add_function(wrap_pyfunction!(py_count_overlaps, m)?)?;
        Ok(())
    }
}
