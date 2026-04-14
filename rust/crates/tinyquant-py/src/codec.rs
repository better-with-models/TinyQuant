//! PyO3 bindings for the `tinyquant-core` codec layer.
//!
//! Surface mirrors `tinyquant_cpu.codec` one-for-one. All value objects are
//! `#[pyclass(frozen, eq, hash)]` so Python-side mutation raises
//! `AttributeError` — matching the Python reference's `@dataclass(frozen=True)`.

use numpy::{PyArray1, PyReadonlyArray1, PyReadonlyArray2};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyBytes;

use tinyquant_core::codec::{
    compress as core_compress, decompress as core_decompress, Codebook as CoreCodebook,
    Codec as CoreCodec, CodecConfig as CoreCodecConfig, CompressedVector as CoreCompressedVector,
    RotationMatrix as CoreRotationMatrix,
};
use tinyquant_io::{from_bytes as io_from_bytes, to_bytes as io_to_bytes};

use crate::errors::{map_codec_error, CodebookIncompatibleError};
use crate::numpy_bridge::{array1_as_slice, array2_as_slice, vec_to_py_array1};

// ---------------------------------------------------------------------------
// CodecConfig
// ---------------------------------------------------------------------------

/// Immutable value object describing codec parameters.
#[pyclass(name = "CodecConfig", module = "tinyquant_rs", frozen, eq, hash)]
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct PyCodecConfig {
    pub(crate) inner: CoreCodecConfig,
}

#[pymethods]
impl PyCodecConfig {
    #[new]
    #[pyo3(signature = (bit_width, seed, dimension, residual_enabled = true))]
    fn new(bit_width: u8, seed: u64, dimension: u32, residual_enabled: bool) -> PyResult<Self> {
        let inner = CoreCodecConfig::new(bit_width, seed, dimension, residual_enabled)
            .map_err(map_codec_error)?;
        Ok(Self { inner })
    }

    #[getter]
    fn bit_width(&self) -> u8 {
        self.inner.bit_width()
    }

    #[getter]
    fn seed(&self) -> u64 {
        self.inner.seed()
    }

    #[getter]
    fn dimension(&self) -> u32 {
        self.inner.dimension()
    }

    #[getter]
    fn residual_enabled(&self) -> bool {
        self.inner.residual_enabled()
    }

    #[getter]
    fn num_codebook_entries(&self) -> u32 {
        self.inner.num_codebook_entries()
    }

    #[getter]
    fn config_hash(&self) -> String {
        self.inner.config_hash().to_string()
    }

    fn __repr__(&self) -> String {
        format!(
            "CodecConfig(bit_width={}, seed={}, dimension={}, residual_enabled={})",
            self.inner.bit_width(),
            self.inner.seed(),
            self.inner.dimension(),
            if self.inner.residual_enabled() {
                "True"
            } else {
                "False"
            },
        )
    }
}

// ---------------------------------------------------------------------------
// Codebook
// ---------------------------------------------------------------------------

/// Immutable lookup table mapping quantized indices to float32 values.
#[pyclass(name = "Codebook", module = "tinyquant_rs", frozen)]
#[derive(Clone)]
pub struct PyCodebook {
    pub(crate) inner: CoreCodebook,
}

#[pymethods]
impl PyCodebook {
    #[new]
    fn new<'py>(entries: PyReadonlyArray1<'py, f32>, bit_width: u8) -> PyResult<Self> {
        let slice = array1_as_slice(&entries)?;
        let boxed: Box<[f32]> = slice.to_vec().into_boxed_slice();
        let inner = CoreCodebook::new(boxed, bit_width).map_err(map_codec_error)?;
        Ok(Self { inner })
    }

    /// Train a codebook from representative FP32 vectors.
    #[classmethod]
    fn train<'py>(
        _cls: &Bound<'_, pyo3::types::PyType>,
        py: Python<'py>,
        vectors: PyReadonlyArray2<'py, f32>,
        config: &PyCodecConfig,
    ) -> PyResult<Self> {
        let (slice, _rows, _cols) = array2_as_slice(&vectors)?;
        let cfg = config.inner.clone();
        let inner = py
            .allow_threads(move || CoreCodebook::train(slice, &cfg))
            .map_err(map_codec_error)?;
        Ok(Self { inner })
    }

    #[getter]
    fn entries<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray1<f32>> {
        PyArray1::from_slice_bound(py, self.inner.entries())
    }

    #[getter]
    fn bit_width(&self) -> u8 {
        self.inner.bit_width()
    }

    #[getter]
    fn num_entries(&self) -> u32 {
        self.inner.num_entries()
    }
}

// ---------------------------------------------------------------------------
// RotationMatrix
// ---------------------------------------------------------------------------

/// Deterministic orthogonal rotation matrix for vector preconditioning.
#[pyclass(name = "RotationMatrix", module = "tinyquant_rs", frozen)]
#[derive(Clone)]
pub struct PyRotationMatrix {
    inner: CoreRotationMatrix,
}

#[pymethods]
impl PyRotationMatrix {
    /// Build (or fetch from cache) a rotation matrix from the given config.
    #[classmethod]
    fn from_config(_cls: &Bound<'_, pyo3::types::PyType>, config: &PyCodecConfig) -> Self {
        Self {
            inner: CoreRotationMatrix::from_config(&config.inner),
        }
    }

    #[getter]
    fn seed(&self) -> u64 {
        self.inner.seed()
    }

    #[getter]
    fn dimension(&self) -> u32 {
        self.inner.dimension()
    }

    fn apply<'py>(
        &self,
        py: Python<'py>,
        vector: PyReadonlyArray1<'py, f32>,
    ) -> PyResult<Bound<'py, PyArray1<f32>>> {
        let slice = array1_as_slice(&vector)?;
        let dim = self.inner.dimension() as usize;
        if slice.len() != dim {
            return Err(PyValueError::new_err(format!(
                "vector dimension must be {dim}, got {}",
                slice.len()
            )));
        }
        let mut out = vec![0.0_f32; dim];
        self.inner
            .apply_into(slice, &mut out)
            .map_err(map_codec_error)?;
        Ok(vec_to_py_array1(py, out))
    }

    fn apply_inverse<'py>(
        &self,
        py: Python<'py>,
        vector: PyReadonlyArray1<'py, f32>,
    ) -> PyResult<Bound<'py, PyArray1<f32>>> {
        let slice = array1_as_slice(&vector)?;
        let dim = self.inner.dimension() as usize;
        if slice.len() != dim {
            return Err(PyValueError::new_err(format!(
                "vector dimension must be {dim}, got {}",
                slice.len()
            )));
        }
        let mut out = vec![0.0_f32; dim];
        self.inner
            .apply_inverse_into(slice, &mut out)
            .map_err(map_codec_error)?;
        Ok(vec_to_py_array1(py, out))
    }
}

// ---------------------------------------------------------------------------
// CompressedVector
// ---------------------------------------------------------------------------

/// Immutable output of a codec compression pass.
#[pyclass(name = "CompressedVector", module = "tinyquant_rs", frozen)]
#[derive(Clone)]
pub struct PyCompressedVector {
    pub(crate) inner: CoreCompressedVector,
}

#[pymethods]
impl PyCompressedVector {
    #[getter]
    fn indices<'py>(&self, py: Python<'py>) -> Bound<'py, numpy::PyArray1<u8>> {
        PyArray1::from_slice_bound(py, self.inner.indices())
    }

    #[getter]
    fn residual<'py>(&self, py: Python<'py>) -> Option<Bound<'py, PyBytes>> {
        self.inner.residual().map(|r| PyBytes::new_bound(py, r))
    }

    #[getter]
    fn config_hash(&self) -> String {
        self.inner.config_hash().to_string()
    }

    #[getter]
    fn dimension(&self) -> u32 {
        self.inner.dimension()
    }

    #[getter]
    fn bit_width(&self) -> u8 {
        self.inner.bit_width()
    }

    #[getter]
    fn has_residual(&self) -> bool {
        self.inner.has_residual()
    }

    #[getter]
    fn size_bytes(&self) -> usize {
        self.inner.size_bytes()
    }

    /// Serialize to compact binary form.
    fn to_bytes<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        let bytes = io_to_bytes(&self.inner);
        PyBytes::new_bound(py, &bytes)
    }

    /// Deserialize from compact binary form.
    #[classmethod]
    fn from_bytes(_cls: &Bound<'_, pyo3::types::PyType>, data: &[u8]) -> PyResult<Self> {
        let inner = io_from_bytes(data).map_err(|err| {
            PyValueError::new_err(format!("failed to decode CompressedVector: {err}"))
        })?;
        Ok(Self { inner })
    }
}

// ---------------------------------------------------------------------------
// Codec service
// ---------------------------------------------------------------------------

/// Stateless codec service — wraps `tinyquant_core::codec::Codec`.
#[pyclass(name = "Codec", module = "tinyquant_rs", frozen)]
#[derive(Clone, Copy, Default)]
pub struct PyCodec;

#[pymethods]
impl PyCodec {
    #[new]
    fn new() -> Self {
        Self
    }

    fn compress<'py>(
        &self,
        py: Python<'py>,
        vector: PyReadonlyArray1<'py, f32>,
        config: &PyCodecConfig,
        codebook: &PyCodebook,
    ) -> PyResult<PyCompressedVector> {
        let slice = array1_as_slice(&vector)?;
        let cfg = config.inner.clone();
        let cb = codebook.inner.clone();
        let buf: Vec<f32> = slice.to_vec();
        let cv = py
            .allow_threads(move || CoreCodec::new().compress(&buf, &cfg, &cb))
            .map_err(map_codec_error)?;
        Ok(PyCompressedVector { inner: cv })
    }

    fn decompress<'py>(
        &self,
        py: Python<'py>,
        compressed: &PyCompressedVector,
        config: &PyCodecConfig,
        codebook: &PyCodebook,
    ) -> PyResult<Bound<'py, PyArray1<f32>>> {
        let cfg = config.inner.clone();
        let cb = codebook.inner.clone();
        let cv = compressed.inner.clone();
        let out = py
            .allow_threads(move || CoreCodec::new().decompress(&cv, &cfg, &cb))
            .map_err(map_codec_error)?;
        Ok(vec_to_py_array1(py, out))
    }

    fn compress_batch<'py>(
        &self,
        py: Python<'py>,
        vectors: PyReadonlyArray2<'py, f32>,
        config: &PyCodecConfig,
        codebook: &PyCodebook,
    ) -> PyResult<Vec<PyCompressedVector>> {
        let (slice, rows, cols) = array2_as_slice(&vectors)?;
        let cfg = config.inner.clone();
        let cb = codebook.inner.clone();
        // Copy the slice while we still own the GIL; only then release it for
        // the compute portion.
        let buf: Vec<f32> = slice.to_vec();
        let cvs = py
            .allow_threads(move || CoreCodec::new().compress_batch(&buf, rows, cols, &cfg, &cb))
            .map_err(map_codec_error)?;
        Ok(cvs
            .into_iter()
            .map(|cv| PyCompressedVector { inner: cv })
            .collect())
    }

    fn decompress_batch<'py>(
        &self,
        py: Python<'py>,
        compressed: Vec<PyCompressedVector>,
        config: &PyCodecConfig,
        codebook: &PyCodebook,
    ) -> PyResult<Bound<'py, numpy::PyArray2<f32>>> {
        if codebook.inner.bit_width() != config.inner.bit_width() {
            return Err(CodebookIncompatibleError::new_err(format!(
                "codebook bit_width {} does not match config bit_width {}",
                codebook.inner.bit_width(),
                config.inner.bit_width(),
            )));
        }
        let cfg = config.inner.clone();
        let cb = codebook.inner.clone();
        let dim = cfg.dimension() as usize;
        let n = compressed.len();
        let inners: Vec<CoreCompressedVector> = compressed.into_iter().map(|c| c.inner).collect();
        let mut out = vec![0.0_f32; n * dim];
        py.allow_threads(|| CoreCodec::new().decompress_batch_into(&inners, &cfg, &cb, &mut out))
            .map_err(map_codec_error)?;
        Ok(numpy::PyArray2::from_vec2_bound(
            py,
            &out.chunks_exact(dim)
                .map(<[f32]>::to_vec)
                .collect::<Vec<_>>(),
        )?)
    }

    fn build_codebook<'py>(
        &self,
        py: Python<'py>,
        training_vectors: PyReadonlyArray2<'py, f32>,
        config: &PyCodecConfig,
    ) -> PyResult<PyCodebook> {
        PyCodebook::train(
            &py.get_type_bound::<PyCodebook>(),
            py,
            training_vectors,
            config,
        )
    }

    fn build_rotation(&self, config: &PyCodecConfig) -> PyRotationMatrix {
        PyRotationMatrix {
            inner: CoreRotationMatrix::from_config(&config.inner),
        }
    }
}

// ---------------------------------------------------------------------------
// Module-level functions
// ---------------------------------------------------------------------------

/// Module-level convenience — equivalent to `Codec().compress(...)`.
#[pyfunction]
pub fn compress<'py>(
    py: Python<'py>,
    vector: PyReadonlyArray1<'py, f32>,
    config: &PyCodecConfig,
    codebook: &PyCodebook,
) -> PyResult<PyCompressedVector> {
    let slice = array1_as_slice(&vector)?;
    let cfg = config.inner.clone();
    let cb = codebook.inner.clone();
    let buf: Vec<f32> = slice.to_vec();
    let cv = py
        .allow_threads(move || core_compress(&buf, &cfg, &cb))
        .map_err(map_codec_error)?;
    Ok(PyCompressedVector { inner: cv })
}

/// Module-level convenience — equivalent to `Codec().decompress(...)`.
#[pyfunction]
pub fn decompress<'py>(
    py: Python<'py>,
    compressed: &PyCompressedVector,
    config: &PyCodecConfig,
    codebook: &PyCodebook,
) -> PyResult<Bound<'py, PyArray1<f32>>> {
    let cfg = config.inner.clone();
    let cb = codebook.inner.clone();
    let cv = compressed.inner.clone();
    let out = py
        .allow_threads(move || core_decompress(&cv, &cfg, &cb))
        .map_err(map_codec_error)?;
    Ok(vec_to_py_array1(py, out))
}
