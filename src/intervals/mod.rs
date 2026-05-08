use pyo3::prelude::*;

mod ops;
mod schema;

#[pymodule(submodule)]
pub mod intervals {
    use pyo3::prelude::*;

    use super::ops::{py_nearest, py_overlap};

    #[pymodule_init]
    fn init(m: &Bound<'_, PyModule>) -> PyResult<()> {
        m.add_function(wrap_pyfunction!(py_overlap, m)?)?;
        m.add_function(wrap_pyfunction!(py_nearest, m)?)?;
        Ok(())
    }
}
