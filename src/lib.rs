use pyo3::prelude::*;

pub mod arrow_utils;
mod intervals;
mod io;

#[pymodule]
mod _ladle {
    use pyo3::prelude::*;

    #[pymodule_export]
    use super::io::io;
    #[pymodule_export]
    use super::intervals::intervals;

    const VERSION: &str = env!("CARGO_PKG_VERSION");

    #[pymodule_init]
    fn init(m: &Bound<'_, PyModule>) -> PyResult<()> {
        m.add("VERSION", VERSION)?;
        Ok(())
    }
}
