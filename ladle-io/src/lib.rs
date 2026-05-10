use pyo3::prelude::*;

pub use ladle_arrow as arrow_utils;
mod io;

#[pymodule]
mod _ladle_io {
    use pyo3::prelude::*;

    #[pymodule_export]
    use super::io::io;

    const VERSION: &str = env!("CARGO_PKG_VERSION");

    #[pymodule_init]
    fn init(m: &Bound<'_, PyModule>) -> PyResult<()> {
        m.add("VERSION", VERSION)?;
        Ok(())
    }
}
