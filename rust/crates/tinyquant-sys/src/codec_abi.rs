//! Codec layer of the TinyQuant C ABI.
//!
//! Every `extern "C" fn` in this module follows the same shape:
//!
//! 1. Clear / zero the caller-supplied `TinyQuantError*` (if any).
//! 2. Wrap the body in `std::panic::catch_unwind(AssertUnwindSafe(|| ...))`.
//! 3. Validate all raw-pointer arguments; null where non-null is required
//!    returns [`TinyQuantErrorKind::InvalidArgument`] or
//!    [`TinyQuantErrorKind::InvalidHandle`].
//! 4. Convert handle pointers via `Box::from_raw` / reference reborrow.
//! 5. Call into `tinyquant-core` / `tinyquant-io`.
//! 6. Map any [`CodecError`] through [`crate::error::set_error`].
//!
//! See `docs/design/rust/ffi-and-bindings.md` §Binding 2 for the design
//! source of truth.
//!
//! # Why this module allows `unsafe_code`
//!
//! Every entry point takes raw pointers from C, and every handle is
//! minted / destroyed with `Box::into_raw` / `Box::from_raw`. Those
//! operations are intrinsically `unsafe`. The `#![allow(unsafe_code)]`
//! is scoped to this file only; all other modules inherit the crate's
//! `#![forbid(unsafe_code)]` attribute.
#![allow(unsafe_code)]
// The ABI layer uses `as` casts heavily to shuttle values across the C
// boundary. These are well-understood and documented inline.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss
)]

use alloc::boxed::Box;
use core::ffi::c_char;
use core::panic::AssertUnwindSafe;
use core::ptr;
use core::slice;
use std::panic;

use tinyquant_core::codec::{Codebook, Codec, CodecConfig, CompressedVector};
use tinyquant_io::compressed_vector;

use crate::error::{
    set_error, set_invalid_argument, set_invalid_handle, set_panic_error, TinyQuantError,
    TinyQuantErrorKind,
};
use crate::handle::{CodebookHandle, CodecConfigHandle, CompressedVectorHandle};
use crate::internal::CodecConfigBox;

// ── CodecConfig ───────────────────────────────────────────────────────

/// Construct a new [`CodecConfig`] and return an opaque handle through
/// `out`.
///
/// On success, `*out` owns the handle; the caller must pair every
/// successful `tq_codec_config_new` call with a `tq_codec_config_free`.
///
/// # Errors
///
/// - `InvalidArgument` if `out` or `err` is null.
/// - `InvalidArgument` for unsupported `bit_width` or zero `dimension`.
///
/// # Safety
///
/// - `out`, if non-null, must point to a writable `*mut CodecConfigHandle`.
/// - `err`, if non-null, must point to a valid `TinyQuantError` slot.
#[no_mangle]
pub unsafe extern "C" fn tq_codec_config_new(
    bit_width: u8,
    seed: u64,
    dimension: u32,
    residual_enabled: bool,
    out: *mut *mut CodecConfigHandle,
    err: *mut TinyQuantError,
) -> TinyQuantErrorKind {
    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        if out.is_null() {
            return set_invalid_argument(err, "out pointer is null");
        }
        unsafe { *out = ptr::null_mut() };
        match CodecConfig::new(bit_width, seed, dimension, residual_enabled) {
            Ok(cfg) => {
                let boxed = Box::new(CodecConfigBox::new(cfg));
                let raw = Box::into_raw(boxed).cast::<CodecConfigHandle>();
                unsafe { *out = raw };
                TinyQuantErrorKind::Ok
            }
            Err(e) => set_error(err, e),
        }
    }));
    match result {
        Ok(kind) => kind,
        Err(payload) => set_panic_error(err, &*payload),
    }
}

/// Free a handle returned by [`tq_codec_config_new`]. Accepts null.
///
/// # Safety
///
/// - `handle`, if non-null, must be a pointer previously returned by
///   [`tq_codec_config_new`] and not yet freed.
#[no_mangle]
pub unsafe extern "C" fn tq_codec_config_free(handle: *mut CodecConfigHandle) {
    if handle.is_null() {
        return;
    }
    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        let boxed = unsafe { Box::from_raw(handle.cast::<CodecConfigBox>()) };
        drop(boxed);
    }));
    // A panic during drop is swallowed — we have no `TinyQuantError` out
    // slot here, and freeing a handle should never fail. Logging would
    // require a dependency we don't want to add.
    let _ = result;
}

fn with_config<R>(
    handle: *const CodecConfigHandle,
    default: R,
    f: impl FnOnce(&CodecConfig) -> R,
) -> R {
    if handle.is_null() {
        return default;
    }
    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        let cfg = unsafe { &*handle.cast::<CodecConfigBox>() };
        f(&cfg.inner)
    }));
    result.unwrap_or(default)
}

/// Return the bit width embedded in a config handle. Returns 0 on null.
///
/// # Safety
///
/// `handle` must be null or a live pointer from [`tq_codec_config_new`].
#[no_mangle]
pub unsafe extern "C" fn tq_codec_config_bit_width(handle: *const CodecConfigHandle) -> u8 {
    with_config(handle, 0, CodecConfig::bit_width)
}

/// Return the seed embedded in a config handle. Returns 0 on null.
///
/// # Safety
///
/// `handle` must be null or a live pointer from [`tq_codec_config_new`].
#[no_mangle]
pub unsafe extern "C" fn tq_codec_config_seed(handle: *const CodecConfigHandle) -> u64 {
    with_config(handle, 0, CodecConfig::seed)
}

/// Return the dimension embedded in a config handle. Returns 0 on null.
///
/// # Safety
///
/// `handle` must be null or a live pointer from [`tq_codec_config_new`].
#[no_mangle]
pub unsafe extern "C" fn tq_codec_config_dimension(handle: *const CodecConfigHandle) -> u32 {
    with_config(handle, 0, CodecConfig::dimension)
}

/// Return whether residual correction is enabled. Returns false on null.
///
/// # Safety
///
/// `handle` must be null or a live pointer from [`tq_codec_config_new`].
#[no_mangle]
pub unsafe extern "C" fn tq_codec_config_residual_enabled(
    handle: *const CodecConfigHandle,
) -> bool {
    with_config(handle, false, CodecConfig::residual_enabled)
}

/// Return the borrowed NUL-terminated SHA-256 `config_hash`.
///
/// Pointer is valid until the handle is freed. Returns null on null input.
///
/// # Safety
///
/// `handle` must be null or a live pointer from [`tq_codec_config_new`].
#[no_mangle]
pub unsafe extern "C" fn tq_codec_config_hash(handle: *const CodecConfigHandle) -> *const c_char {
    if handle.is_null() {
        return ptr::null();
    }
    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        let cfg = unsafe { &*handle.cast::<CodecConfigBox>() };
        cfg.hash_cstring.as_ptr()
    }));
    result.unwrap_or(ptr::null())
}

// ── Codebook ──────────────────────────────────────────────────────────

/// Train a [`Codebook`] from `training_vectors` (row-major, `rows × cols`).
///
/// # Safety
///
/// - `training_vectors` must point to at least `rows * cols` `f32`s.
/// - `config`, `out`, `err` must satisfy the usual pointer contracts.
#[no_mangle]
pub unsafe extern "C" fn tq_codebook_train(
    training_vectors: *const f32,
    rows: usize,
    cols: usize,
    config: *const CodecConfigHandle,
    out: *mut *mut CodebookHandle,
    err: *mut TinyQuantError,
) -> TinyQuantErrorKind {
    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        if out.is_null() {
            return set_invalid_argument(err, "out pointer is null");
        }
        unsafe { *out = ptr::null_mut() };
        if config.is_null() {
            return set_invalid_handle(err);
        }
        if training_vectors.is_null() {
            return set_invalid_argument(err, "training_vectors pointer is null");
        }
        let Some(total) = rows.checked_mul(cols) else {
            return set_invalid_argument(err, "rows * cols overflows usize");
        };
        let slice = unsafe { slice::from_raw_parts(training_vectors, total) };
        let cfg = unsafe { &(*config.cast::<CodecConfigBox>()).inner };
        match Codebook::train(slice, cfg) {
            Ok(cb) => {
                let raw = Box::into_raw(Box::new(cb)).cast::<CodebookHandle>();
                unsafe { *out = raw };
                TinyQuantErrorKind::Ok
            }
            Err(e) => set_error(err, e),
        }
    }));
    match result {
        Ok(kind) => kind,
        Err(payload) => set_panic_error(err, &*payload),
    }
}

/// Free a codebook handle. Accepts null.
///
/// # Safety
///
/// See [`tq_codec_config_free`].
#[no_mangle]
pub unsafe extern "C" fn tq_codebook_free(handle: *mut CodebookHandle) {
    if handle.is_null() {
        return;
    }
    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        let boxed = unsafe { Box::from_raw(handle.cast::<Codebook>()) };
        drop(boxed);
    }));
    let _ = result;
}

/// Return the bit width of a codebook. Returns 0 on null.
///
/// # Safety
///
/// `handle` must be null or a live pointer from [`tq_codebook_train`].
#[no_mangle]
pub unsafe extern "C" fn tq_codebook_bit_width(handle: *const CodebookHandle) -> u8 {
    if handle.is_null() {
        return 0;
    }
    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        let cb = unsafe { &*handle.cast::<Codebook>() };
        cb.bit_width()
    }));
    result.unwrap_or(0)
}

// ── Codec compress / decompress ───────────────────────────────────────

/// Compress a single vector.
///
/// # Safety
///
/// - `vector` must point to at least `vector_len` `f32`s.
/// - `config`, `codebook`, `out`, `err` must satisfy the usual contracts.
#[no_mangle]
pub unsafe extern "C" fn tq_codec_compress(
    config: *const CodecConfigHandle,
    codebook: *const CodebookHandle,
    vector: *const f32,
    vector_len: usize,
    out: *mut *mut CompressedVectorHandle,
    err: *mut TinyQuantError,
) -> TinyQuantErrorKind {
    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        if out.is_null() {
            return set_invalid_argument(err, "out pointer is null");
        }
        unsafe { *out = ptr::null_mut() };
        if config.is_null() || codebook.is_null() {
            return set_invalid_handle(err);
        }
        if vector.is_null() {
            return set_invalid_argument(err, "vector pointer is null");
        }
        let cfg = unsafe { &(*config.cast::<CodecConfigBox>()).inner };
        let cb = unsafe { &*codebook.cast::<Codebook>() };
        let slice = unsafe { slice::from_raw_parts(vector, vector_len) };
        match Codec::new().compress(slice, cfg, cb) {
            Ok(cv) => {
                let raw = Box::into_raw(Box::new(cv)).cast::<CompressedVectorHandle>();
                unsafe { *out = raw };
                TinyQuantErrorKind::Ok
            }
            Err(e) => set_error(err, e),
        }
    }));
    match result {
        Ok(kind) => kind,
        Err(payload) => set_panic_error(err, &*payload),
    }
}

/// Decompress a [`CompressedVector`] into the caller-supplied buffer
/// (length must equal `config.dimension()`).
///
/// # Safety
///
/// - `out` must point to a writable buffer of `out_len` `f32`s.
/// - Handles must satisfy the usual contracts.
#[no_mangle]
pub unsafe extern "C" fn tq_codec_decompress(
    config: *const CodecConfigHandle,
    codebook: *const CodebookHandle,
    compressed: *const CompressedVectorHandle,
    out: *mut f32,
    out_len: usize,
    err: *mut TinyQuantError,
) -> TinyQuantErrorKind {
    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        if config.is_null() || codebook.is_null() || compressed.is_null() {
            return set_invalid_handle(err);
        }
        if out.is_null() {
            return set_invalid_argument(err, "out pointer is null");
        }
        let cfg = unsafe { &(*config.cast::<CodecConfigBox>()).inner };
        let cb = unsafe { &*codebook.cast::<Codebook>() };
        let cv = unsafe { &*compressed.cast::<CompressedVector>() };
        let buf = unsafe { slice::from_raw_parts_mut(out, out_len) };
        match Codec::new().decompress_into(cv, cfg, cb, buf) {
            Ok(()) => TinyQuantErrorKind::Ok,
            Err(e) => set_error(err, e),
        }
    }));
    match result {
        Ok(kind) => kind,
        Err(payload) => set_panic_error(err, &*payload),
    }
}

// ── CompressedVector lifecycle + serialization ────────────────────────

/// Return the dimension of a compressed vector. Returns 0 on null.
///
/// # Safety
///
/// `handle` must be null or a live `CompressedVectorHandle`.
#[no_mangle]
pub unsafe extern "C" fn tq_compressed_vector_dimension(
    handle: *const CompressedVectorHandle,
) -> u32 {
    if handle.is_null() {
        return 0;
    }
    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        let cv = unsafe { &*handle.cast::<CompressedVector>() };
        cv.dimension()
    }));
    result.unwrap_or(0)
}

/// Return the bit width of a compressed vector. Returns 0 on null.
///
/// # Safety
///
/// `handle` must be null or a live `CompressedVectorHandle`.
#[no_mangle]
pub unsafe extern "C" fn tq_compressed_vector_bit_width(
    handle: *const CompressedVectorHandle,
) -> u8 {
    if handle.is_null() {
        return 0;
    }
    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        let cv = unsafe { &*handle.cast::<CompressedVector>() };
        cv.bit_width()
    }));
    result.unwrap_or(0)
}

/// Free a compressed vector handle. Accepts null.
///
/// # Safety
///
/// `handle`, if non-null, must be a pointer previously returned by
/// [`tq_codec_compress`] or [`tq_compressed_vector_from_bytes`].
#[no_mangle]
pub unsafe extern "C" fn tq_compressed_vector_free(handle: *mut CompressedVectorHandle) {
    if handle.is_null() {
        return;
    }
    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        let boxed = unsafe { Box::from_raw(handle.cast::<CompressedVector>()) };
        drop(boxed);
    }));
    let _ = result;
}

/// Serialise a [`CompressedVector`] to the wire format.
///
/// On success, `*out_bytes` holds a freshly allocated buffer owned by
/// Rust; release it with [`tq_bytes_free`].
///
/// # Safety
///
/// - `handle` must be a live compressed-vector handle.
/// - `out_bytes` and `out_len` must be writable.
#[no_mangle]
pub unsafe extern "C" fn tq_compressed_vector_to_bytes(
    handle: *const CompressedVectorHandle,
    out_bytes: *mut *mut u8,
    out_len: *mut usize,
    err: *mut TinyQuantError,
) -> TinyQuantErrorKind {
    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        if handle.is_null() {
            return set_invalid_handle(err);
        }
        if out_bytes.is_null() || out_len.is_null() {
            return set_invalid_argument(err, "out_bytes / out_len null");
        }
        unsafe {
            *out_bytes = ptr::null_mut();
            *out_len = 0;
        }
        let cv = unsafe { &*handle.cast::<CompressedVector>() };
        let bytes: alloc::vec::Vec<u8> = compressed_vector::to_bytes(cv);
        let len = bytes.len();
        // Use a boxed slice so we can recover the allocator layout via
        // `Box::from_raw` in `tq_bytes_free` (Vec -> Box<[u8]> keeps the
        // exact allocation; we return pointer + length as a pair).
        let boxed: Box<[u8]> = bytes.into_boxed_slice();
        let raw = Box::into_raw(boxed).cast::<u8>();
        unsafe {
            *out_bytes = raw;
            *out_len = len;
        }
        TinyQuantErrorKind::Ok
    }));
    match result {
        Ok(kind) => kind,
        Err(payload) => set_panic_error(err, &*payload),
    }
}

/// Deserialise bytes into a [`CompressedVector`]. Returns a new handle.
///
/// # Safety
///
/// - `data` must point to at least `data_len` bytes.
/// - `out`, `err` must satisfy the usual contracts.
#[no_mangle]
pub unsafe extern "C" fn tq_compressed_vector_from_bytes(
    data: *const u8,
    data_len: usize,
    out: *mut *mut CompressedVectorHandle,
    err: *mut TinyQuantError,
) -> TinyQuantErrorKind {
    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        if out.is_null() {
            return set_invalid_argument(err, "out pointer is null");
        }
        unsafe { *out = ptr::null_mut() };
        if data.is_null() && data_len != 0 {
            return set_invalid_argument(err, "data pointer is null");
        }
        let slice = if data.is_null() {
            &[][..]
        } else {
            unsafe { slice::from_raw_parts(data, data_len) }
        };
        match compressed_vector::from_bytes(slice) {
            Ok(cv) => {
                let raw = Box::into_raw(Box::new(cv)).cast::<CompressedVectorHandle>();
                unsafe { *out = raw };
                TinyQuantErrorKind::Ok
            }
            Err(e) => set_error(err, e),
        }
    }));
    match result {
        Ok(kind) => kind,
        Err(payload) => set_panic_error(err, &*payload),
    }
}

/// Free a byte buffer previously returned by
/// [`tq_compressed_vector_to_bytes`].
///
/// # Safety
///
/// `ptr` / `len`, if `ptr` is non-null, must be the exact pair produced
/// by `tq_compressed_vector_to_bytes`.
#[no_mangle]
pub unsafe extern "C" fn tq_bytes_free(ptr: *mut u8, len: usize) {
    if ptr.is_null() {
        return;
    }
    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        // Reconstruct the boxed slice so the original allocator layout is
        // preserved (Vec::from_raw_parts with a wrong cap would be UB).
        let slice_ptr: *mut [u8] = core::ptr::slice_from_raw_parts_mut(ptr, len);
        let boxed: Box<[u8]> = unsafe { Box::from_raw(slice_ptr) };
        drop(boxed);
    }));
    let _ = result;
}

// ── Version helpers ───────────────────────────────────────────────────

/// Return the crate version as a borrowed NUL-terminated C string.
///
/// The returned pointer points into a `'static` string literal with an
/// embedded NUL terminator, so it is safe to share across threads and
/// lives for the program's lifetime.
///
/// # Safety
///
/// The function itself is safe to call; it is declared `unsafe extern`
/// only to match the ABI convention documented in the module header.
#[no_mangle]
pub const extern "C" fn tq_version() -> *const c_char {
    const VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), "\0");
    VERSION.as_ptr().cast::<c_char>()
}

/// Historical helper retained for Phase 11 callers. Returns 0 and will
/// be removed once all call sites migrate to [`tq_version`].
#[no_mangle]
pub const extern "C" fn tinyquant_version() -> u32 {
    0
}

// ── Panic-crossing probe (test only) ──────────────────────────────────

/// Deliberately panic inside an `extern "C" fn` to exercise the
/// `catch_unwind` wrapper from integration tests. Not part of the
/// published C ABI header (hidden via `cbindgen.toml` except list).
///
/// # Panics
///
/// Always panics internally. The panic is *always* caught by the
/// surrounding `catch_unwind` and mapped to
/// [`TinyQuantErrorKind::Panic`]; no unwinding ever crosses the FFI
/// boundary. That is the point of the probe.
///
/// # Safety
///
/// - `err`, if non-null, must point to a valid `TinyQuantError` slot
///   that the caller owns. The function writes the `Panic` discriminant
///   and an owned message pointer that must be freed with
///   [`crate::error::tq_error_free`].
#[cfg(any(test, feature = "panic-probe"))]
#[no_mangle]
#[allow(clippy::panic)]
pub unsafe extern "C" fn tq_test_panic(err: *mut TinyQuantError) -> TinyQuantErrorKind {
    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        panic!("deliberate panic from tq_test_panic");
    }));
    match result {
        Ok(()) => TinyQuantErrorKind::Ok,
        Err(payload) => set_panic_error(err, &*payload),
    }
}
