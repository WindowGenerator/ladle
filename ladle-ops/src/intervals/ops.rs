use pyo3::exceptions::PyIOError;
use pyo3::prelude::*;

use crate::arrow_utils::{batch_to_pyarrow, pyarrow_to_batch};

use super::pairwise::{count_overlaps_batches, nearest_batches, overlap_batches};
use super::ranges::{intersect_ranges_batches, setdiff_ranges_batches, union_ranges_batches};
use super::single::{
    cluster_batches, complement_batches, coverage_batches, disjoin_batches, expand_batches,
    flank_batches, merge_batches, set_width_batches, shift_batches, sort_bedframe_batches,
    subtract_batches, tile_batches,
};

#[pyfunction(name = "overlap")]
pub fn py_overlap<'py>(
    py: Python<'py>,
    a: &Bound<'_, PyAny>,
    b: &Bound<'_, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let batch_a = pyarrow_to_batch(py, a)?;
    let batch_b = pyarrow_to_batch(py, b)?;
    let result = py
        .detach(|| overlap_batches(&batch_a, &batch_b))
        .map_err(|e| PyIOError::new_err(e.to_string()))?;
    batch_to_pyarrow(py, result)
}

#[pyfunction(name = "nearest")]
pub fn py_nearest<'py>(
    py: Python<'py>,
    query: &Bound<'_, PyAny>,
    target: &Bound<'_, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let batch_q = pyarrow_to_batch(py, query)?;
    let batch_t = pyarrow_to_batch(py, target)?;
    let result = py
        .detach(|| nearest_batches(&batch_q, &batch_t))
        .map_err(|e| PyIOError::new_err(e.to_string()))?;
    batch_to_pyarrow(py, result)
}

#[pyfunction(name = "count_overlaps")]
pub fn py_count_overlaps<'py>(
    py: Python<'py>,
    a: &Bound<'_, PyAny>,
    b: &Bound<'_, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let batch_a = pyarrow_to_batch(py, a)?;
    let batch_b = pyarrow_to_batch(py, b)?;
    let result = py
        .detach(|| count_overlaps_batches(&batch_a, &batch_b))
        .map_err(|e| PyIOError::new_err(e.to_string()))?;
    batch_to_pyarrow(py, result)
}

#[pyfunction(name = "cluster")]
pub fn py_cluster<'py>(py: Python<'py>, a: &Bound<'_, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let batch = pyarrow_to_batch(py, a)?;
    let result = py
        .detach(|| cluster_batches(&batch))
        .map_err(|e| PyIOError::new_err(e.to_string()))?;
    batch_to_pyarrow(py, result)
}

#[pyfunction(name = "merge")]
pub fn py_merge<'py>(py: Python<'py>, a: &Bound<'_, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let batch = pyarrow_to_batch(py, a)?;
    let result = py
        .detach(|| merge_batches(&batch))
        .map_err(|e| PyIOError::new_err(e.to_string()))?;
    batch_to_pyarrow(py, result)
}

#[pyfunction(name = "subtract")]
pub fn py_subtract<'py>(
    py: Python<'py>,
    a: &Bound<'_, PyAny>,
    b: &Bound<'_, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let batch_a = pyarrow_to_batch(py, a)?;
    let batch_b = pyarrow_to_batch(py, b)?;
    let result = py
        .detach(|| subtract_batches(&batch_a, &batch_b))
        .map_err(|e| PyIOError::new_err(e.to_string()))?;
    batch_to_pyarrow(py, result)
}

#[pyfunction(name = "complement")]
#[pyo3(signature = (a, chrom_sizes=None))]
pub fn py_complement<'py>(
    py: Python<'py>,
    a: &Bound<'_, PyAny>,
    chrom_sizes: Option<std::collections::HashMap<String, i32>>,
) -> PyResult<Bound<'py, PyAny>> {
    let batch = pyarrow_to_batch(py, a)?;
    let result = py
        .detach(|| complement_batches(&batch, chrom_sizes.as_ref()))
        .map_err(|e| PyIOError::new_err(e.to_string()))?;
    batch_to_pyarrow(py, result)
}

#[pyfunction(name = "coverage")]
pub fn py_coverage<'py>(py: Python<'py>, a: &Bound<'_, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let batch = pyarrow_to_batch(py, a)?;
    let result = py
        .detach(|| coverage_batches(&batch))
        .map_err(|e| PyIOError::new_err(e.to_string()))?;
    batch_to_pyarrow(py, result)
}

#[pyfunction(name = "expand")]
#[pyo3(signature = (a, amount=0, start_amount=None, end_amount=None))]
pub fn py_expand<'py>(
    py: Python<'py>,
    a: &Bound<'_, PyAny>,
    amount: i32,
    start_amount: Option<i32>,
    end_amount: Option<i32>,
) -> PyResult<Bound<'py, PyAny>> {
    let batch = pyarrow_to_batch(py, a)?;
    let result = py
        .detach(|| expand_batches(&batch, amount, start_amount, end_amount))
        .map_err(|e| PyIOError::new_err(e.to_string()))?;
    batch_to_pyarrow(py, result)
}

#[pyfunction(name = "shift")]
pub fn py_shift<'py>(
    py: Python<'py>,
    a: &Bound<'_, PyAny>,
    amount: i32,
) -> PyResult<Bound<'py, PyAny>> {
    let batch = pyarrow_to_batch(py, a)?;
    let result = py
        .detach(|| shift_batches(&batch, amount))
        .map_err(|e| PyIOError::new_err(e.to_string()))?;
    batch_to_pyarrow(py, result)
}

#[pyfunction(name = "sort_bedframe")]
#[pyo3(signature = (a, natural_chrom_order=true))]
pub fn py_sort_bedframe<'py>(
    py: Python<'py>,
    a: &Bound<'_, PyAny>,
    natural_chrom_order: bool,
) -> PyResult<Bound<'py, PyAny>> {
    let batch = pyarrow_to_batch(py, a)?;
    let result = py
        .detach(|| sort_bedframe_batches(&batch, natural_chrom_order))
        .map_err(|e| PyIOError::new_err(e.to_string()))?;
    batch_to_pyarrow(py, result)
}

#[pyfunction(name = "flank")]
#[pyo3(signature = (a, width, start=true))]
pub fn py_flank<'py>(
    py: Python<'py>,
    a: &Bound<'_, PyAny>,
    width: i32,
    start: bool,
) -> PyResult<Bound<'py, PyAny>> {
    let batch = pyarrow_to_batch(py, a)?;
    let result = py
        .detach(|| flank_batches(&batch, width, start))
        .map_err(|e| PyIOError::new_err(e.to_string()))?;
    batch_to_pyarrow(py, result)
}

#[pyfunction(name = "set_width")]
#[pyo3(signature = (a, width, anchor = "start"))]
pub fn py_set_width<'py>(
    py: Python<'py>,
    a: &Bound<'_, PyAny>,
    width: i32,
    anchor: &str,
) -> PyResult<Bound<'py, PyAny>> {
    let batch = pyarrow_to_batch(py, a)?;
    let result = py
        .detach(|| set_width_batches(&batch, width, anchor))
        .map_err(|e| PyIOError::new_err(e.to_string()))?;
    batch_to_pyarrow(py, result)
}

#[pyfunction(name = "tile")]
pub fn py_tile<'py>(
    py: Python<'py>,
    a: &Bound<'_, PyAny>,
    width: i32,
) -> PyResult<Bound<'py, PyAny>> {
    let batch = pyarrow_to_batch(py, a)?;
    let result = py
        .detach(|| tile_batches(&batch, width))
        .map_err(|e| PyIOError::new_err(e.to_string()))?;
    batch_to_pyarrow(py, result)
}

#[pyfunction(name = "disjoin")]
pub fn py_disjoin<'py>(py: Python<'py>, a: &Bound<'_, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let batch = pyarrow_to_batch(py, a)?;
    let result = py
        .detach(|| disjoin_batches(&batch))
        .map_err(|e| PyIOError::new_err(e.to_string()))?;
    batch_to_pyarrow(py, result)
}

#[pyfunction(name = "intersect_ranges")]
pub fn py_intersect_ranges<'py>(
    py: Python<'py>,
    a: &Bound<'_, PyAny>,
    b: &Bound<'_, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let ba = pyarrow_to_batch(py, a)?;
    let bb = pyarrow_to_batch(py, b)?;
    let result = py
        .detach(|| intersect_ranges_batches(&ba, &bb))
        .map_err(|e| PyIOError::new_err(e.to_string()))?;
    batch_to_pyarrow(py, result)
}

#[pyfunction(name = "union_ranges")]
pub fn py_union_ranges<'py>(
    py: Python<'py>,
    a: &Bound<'_, PyAny>,
    b: &Bound<'_, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let ba = pyarrow_to_batch(py, a)?;
    let bb = pyarrow_to_batch(py, b)?;
    let result = py
        .detach(|| union_ranges_batches(&ba, &bb))
        .map_err(|e| PyIOError::new_err(e.to_string()))?;
    batch_to_pyarrow(py, result)
}

#[pyfunction(name = "setdiff_ranges")]
pub fn py_setdiff_ranges<'py>(
    py: Python<'py>,
    a: &Bound<'_, PyAny>,
    b: &Bound<'_, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let ba = pyarrow_to_batch(py, a)?;
    let bb = pyarrow_to_batch(py, b)?;
    let result = py
        .detach(|| setdiff_ranges_batches(&ba, &bb))
        .map_err(|e| PyIOError::new_err(e.to_string()))?;
    batch_to_pyarrow(py, result)
}
