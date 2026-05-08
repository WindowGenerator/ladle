use pyo3::prelude::*;

mod batch;
mod record;
mod reader;
mod writer;

pub use batch::{PyBcfBatchIterator, PyBcfRecordBatch};
pub use record::PyRecord;
pub use reader::{PyIndexedReader, PyQuery, PyReader};
pub use writer::PyWriter;

#[pymodule(submodule)]
pub mod bcf {
    #[pymodule_export]
    use super::PyRecord as Record;
    #[pymodule_export]
    use super::PyReader as Reader;
    #[pymodule_export]
    use super::PyIndexedReader as IndexedReader;
    #[pymodule_export]
    use super::PyQuery as Query;
    #[pymodule_export]
    use super::PyWriter as Writer;
    #[pymodule_export]
    use super::PyBcfRecordBatch as RecordBatch;
    #[pymodule_export]
    use super::PyBcfBatchIterator as RecordBatchIterator;
}
