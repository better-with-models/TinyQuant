//! Zero-copy helpers for passing NumPy `float32` arrays into the Rust core.
//!
//! See [[design/rust/ffi-and-bindings]] §Binding 1 — NumPy interop.

use numpy::{PyArray1, PyReadonlyArray1, PyReadonlyArray2, PyUntypedArrayMethods};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

/// Borrow a 1-D `float32` NumPy array as a `&[f32]` slice without copying.
///
/// Returns a PyValueError if the array is not C-contiguous.
pub(crate) fn array1_as_slice<'py>(arr: &'py PyReadonlyArray1<'py, f32>) -> PyResult<&'py [f32]> {
    arr.as_slice()
        .map_err(|err| PyValueError::new_err(format!("numpy slice unavailable: {err}")))
}

/// Borrow a 2-D `float32` NumPy array as `(slice, rows, cols)` without
/// copying. The array must be C-contiguous and row-major.
pub(crate) fn array2_as_slice<'py>(
    arr: &'py PyReadonlyArray2<'py, f32>,
) -> PyResult<(&'py [f32], usize, usize)> {
    let shape = arr.shape();
    if shape.len() != 2 {
        return Err(PyValueError::new_err("expected a 2-D array of float32"));
    }
    let rows = shape[0];
    let cols = shape[1];
    let slice = arr
        .as_slice()
        .map_err(|_| PyValueError::new_err("input matrix must be C-contiguous float32"))?;
    Ok((slice, rows, cols))
}

/// Construct a new 1-D `PyArray1<f32>` from an owned `Vec<f32>`.
pub(crate) fn vec_to_py_array1(py: Python<'_>, data: Vec<f32>) -> Bound<'_, PyArray1<f32>> {
    PyArray1::from_vec_bound(py, data)
}
