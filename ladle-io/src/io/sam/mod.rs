use pyo3::prelude::*;

mod batch;
pub mod flags;
pub mod header;
pub mod mapping_quality;
pub mod record;
mod reader;
mod writer;

pub use batch::{PySamBatchIterator, PySamRecordBatch};
pub use flags::PyFlags;
pub use header::PyHeader;
pub use mapping_quality::PyMappingQuality;
pub use record::PyRecord;
pub use reader::PyReader;
pub use writer::PyWriter;

#[pymodule(submodule)]
pub mod sam {
    #[pymodule_export]
    use super::PyFlags as Flags;
    #[pymodule_export]
    use super::PyHeader as Header;
    #[pymodule_export]
    use super::PyMappingQuality as MappingQuality;
    #[pymodule_export]
    use super::PyRecord as Record;
    #[pymodule_export]
    use super::PyReader as Reader;
    #[pymodule_export]
    use super::PyWriter as Writer;
    #[pymodule_export]
    use super::PySamRecordBatch as RecordBatch;
    #[pymodule_export]
    use super::PySamBatchIterator as RecordBatchIterator;
}
