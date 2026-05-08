use pyo3::prelude::*;

pub mod bai;
mod batch;
mod record;
mod reader;
mod writer;

pub use batch::{PyBamBatchIterator, PyBamRecordBatch};
pub use record::PyRecord;
pub use reader::{PyIndexedReader, PyQuery, PyReader};
pub use writer::PyWriter;

#[pymodule(submodule)]
pub mod bam {
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
    use super::PyBamRecordBatch as RecordBatch;
    #[pymodule_export]
    use super::PyBamBatchIterator as RecordBatchIterator;
    #[pymodule_export]
    use super::bai::bai;
}
