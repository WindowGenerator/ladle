use pyo3::prelude::*;

mod batch;
pub mod header;
mod reader;
pub mod record;
pub mod schema;
mod writer;

pub use batch::{PyVcfBatchIterator, PyVcfRecordBatch};
pub use header::PyHeader;
pub use reader::PyReader;
pub use record::PyRecord;
pub use writer::PyWriter;

#[pymodule(submodule)]
pub mod vcf {
    #[pymodule_export]
    use super::PyHeader as Header;
    #[pymodule_export]
    use super::PyReader as Reader;
    #[pymodule_export]
    use super::PyRecord as Record;
    #[pymodule_export]
    use super::PyVcfBatchIterator as RecordBatchIterator;
    #[pymodule_export]
    use super::PyVcfRecordBatch as RecordBatch;
    #[pymodule_export]
    use super::PyWriter as Writer;
}
