use pyo3::prelude::*;

mod batch;
mod reader;
pub mod record;
mod writer;

pub use batch::{PyFastqBatchIterator, PyFastqRecordBatch};
pub use reader::PyReader;
pub use record::PyRecord;
pub use writer::PyWriter;

#[pymodule(submodule)]
pub mod fastq {
    #[pymodule_export]
    use super::PyFastqBatchIterator as RecordBatchIterator;
    #[pymodule_export]
    use super::PyFastqRecordBatch as RecordBatch;
    #[pymodule_export]
    use super::PyReader as Reader;
    #[pymodule_export]
    use super::PyRecord as Record;
    #[pymodule_export]
    use super::PyWriter as Writer;
}
