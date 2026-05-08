use pyo3::prelude::*;

mod bam;
mod bcf;
mod bgzf;
mod core;
mod sam;
mod vcf;

#[pymodule(submodule)]
pub mod io {
    #[pymodule_export]
    use super::bam::bam;
    #[pymodule_export]
    use super::bcf::bcf;
    #[pymodule_export]
    use super::bgzf::bgzf;
    #[pymodule_export]
    use super::core::core;
    #[pymodule_export]
    use super::sam::sam;
    #[pymodule_export]
    use super::vcf::vcf;
}
