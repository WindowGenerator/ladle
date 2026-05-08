use pyo3::prelude::*;

pub mod record;
mod reader;
mod writer;

pub use record::PyRecord;
pub use reader::PyReader;
pub use writer::PyWriter;

#[pymodule(submodule)]
pub mod fastq {
    #[pymodule_export]
    use super::PyRecord as Record;
    #[pymodule_export]
    use super::PyReader as Reader;
    #[pymodule_export]
    use super::PyWriter as Writer;
}
