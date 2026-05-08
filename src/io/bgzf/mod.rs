use pyo3::prelude::*;

mod gzi;
mod reader;
mod virtual_position;
mod writer;

pub use reader::PyReader;
pub use virtual_position::PyVirtualPosition;
pub use writer::PyWriter;

#[pymodule(submodule)]
pub mod bgzf {
    use noodles::bgzf::io::writer::CompressionLevel;
    use pyo3::prelude::*;

    #[pymodule_export]
    use super::PyVirtualPosition as VirtualPosition;
    #[pymodule_export]
    use super::PyReader as Reader;
    #[pymodule_export]
    use super::PyWriter as Writer;
    #[pymodule_export]
    use super::gzi::gzi;

    #[pymodule_init]
    fn init(m: &Bound<'_, PyModule>) -> PyResult<()> {
        m.add("COMPRESSION_NONE", CompressionLevel::NONE.get())?;
        m.add("COMPRESSION_FAST", CompressionLevel::FAST.get())?;
        m.add("COMPRESSION_BEST", CompressionLevel::BEST.get())?;
        Ok(())
    }
}
