use pyo3::prelude::*;

mod reader;
pub mod record;
mod writer;

pub use reader::PyReader;
pub use record::PyRecord;
pub use writer::PyWriter;

#[pymodule(submodule)]
pub mod fastq {
    #[pymodule_export]
    use super::PyReader as Reader;
    #[pymodule_export]
    use super::PyRecord as Record;
    #[pymodule_export]
    use super::PyWriter as Writer;
}
