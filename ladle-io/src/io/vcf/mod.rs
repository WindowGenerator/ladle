use pyo3::prelude::*;

mod batch;
pub mod header;
pub mod schema;
pub mod record;
mod reader;
mod writer;

pub use batch::{PyVcfBatchIterator, PyVcfRecordBatch};
pub use header::PyHeader;
pub use record::PyRecord;
pub use reader::PyReader;
pub use writer::PyWriter;

#[pymodule(submodule)]
pub mod vcf {
    #[pymodule_export]
    use super::PyHeader as Header;
    #[pymodule_export]
    use super::PyRecord as Record;
    #[pymodule_export]
    use super::PyReader as Reader;
    #[pymodule_export]
    use super::PyWriter as Writer;
    #[pymodule_export]
    use super::PyVcfRecordBatch as RecordBatch;
    #[pymodule_export]
    use super::PyVcfBatchIterator as RecordBatchIterator;
}
