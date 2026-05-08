use pyo3::prelude::*;

pub mod arrow_utils;
mod intervals;

use intervals::ops::{
    py_cluster, py_complement, py_count_overlaps, py_coverage, py_disjoin,
    py_expand, py_flank, py_merge, py_nearest, py_overlap, py_set_width,
    py_shift, py_sort_bedframe, py_subtract, py_tile,
};

#[pymodule]
mod _ladle_ops {
    use pyo3::prelude::*;

    use super::{
        py_cluster, py_complement, py_count_overlaps, py_coverage, py_disjoin,
        py_expand, py_flank, py_merge, py_nearest, py_overlap, py_set_width,
        py_shift, py_sort_bedframe, py_subtract, py_tile,
    };

    #[pymodule_init]
    fn init(m: &Bound<'_, PyModule>) -> PyResult<()> {
        m.add_function(wrap_pyfunction!(py_overlap, m)?)?;
        m.add_function(wrap_pyfunction!(py_nearest, m)?)?;
        m.add_function(wrap_pyfunction!(py_count_overlaps, m)?)?;
        m.add_function(wrap_pyfunction!(py_cluster, m)?)?;
        m.add_function(wrap_pyfunction!(py_merge, m)?)?;
        m.add_function(wrap_pyfunction!(py_subtract, m)?)?;
        m.add_function(wrap_pyfunction!(py_complement, m)?)?;
        m.add_function(wrap_pyfunction!(py_coverage, m)?)?;
        m.add_function(wrap_pyfunction!(py_expand, m)?)?;
        m.add_function(wrap_pyfunction!(py_shift, m)?)?;
        m.add_function(wrap_pyfunction!(py_sort_bedframe, m)?)?;
        m.add_function(wrap_pyfunction!(py_flank, m)?)?;
        m.add_function(wrap_pyfunction!(py_set_width, m)?)?;
        m.add_function(wrap_pyfunction!(py_tile, m)?)?;
        m.add_function(wrap_pyfunction!(py_disjoin, m)?)?;
        Ok(())
    }
}
