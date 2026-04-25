//! Error types for the TinyQuant C ABI.
//!
//! The C ABI surfaces errors through a pair of structures:
//!
//! - [`TinyQuantErrorKind`] — a `#[repr(C)]` enum with fixed discriminants
//!   that map 1-to-1 onto the Python exception hierarchy frozen by Phase
//!   22.A (`DimensionMismatchError`, `ConfigMismatchError`,
//!   `CodebookIncompatibleError`, `DuplicateVectorError`) plus the
//!   ABI-specific `InvalidHandle`, `InvalidArgument`, `Panic` and
//!   `Unknown` kinds.
//! - [`TinyQuantError`] — an out-pointer structure populated by every
//!   fallible entry point. `message` is an owned `*mut c_char` (allocated
//!   by the Rust `CString` allocator) and MUST be freed by the caller via
//!   [`tq_error_free`] or [`tq_error_free_message`].
//!
//! Internally the module exposes two helpers used by
//! [`crate::codec_abi`] and [`crate::corpus_abi`]:
//! [`set_error`] fills a `TinyQuantError` from a Rust error, and
//! [`set_panic_error`] captures a `catch_unwind` payload.
//!
//! # Why this module allows `unsafe_code`
//!
//! `tq_error_free` / `tq_error_free_message` are `extern "C"` free
//! functions that consume raw pointers owned by C callers. Every other
//! module in `tinyquant-sys` delegates to the handle/abi modules for
//! pointer ownership transfers; here, the allocations originated in
//! Rust and must be released through `CString::from_raw`, which is
//! intrinsically `unsafe`. The allow is scoped to this file only.
#![allow(unsafe_code)]

use alloc::borrow::ToOwned;
use alloc::ffi::CString;
use alloc::format;
use alloc::string::{String, ToString};
use core::any::Any;
use core::ffi::c_char;
use core::ptr;

use tinyquant_core::errors::{BackendError, CodecError, CorpusError};

/// Canonical error discriminants emitted by every `tq_*` entry point.
///
/// Values are fixed for the lifetime of the crate's 0.x line so that C
/// consumers compiled against `tinyquant.h` do not have to recompile on
/// minor-version bumps. Adding a new variant is always allowed; changing
/// an existing discriminant is a breaking change.
#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum TinyQuantErrorKind {
    /// Operation succeeded. `TinyQuantError::message` is guaranteed null.
    #[default]
    Ok = 0,
    /// A handle argument was null or previously freed.
    InvalidHandle = 1,
    /// Vector length / dimension mismatch (maps to Python
    /// `DimensionMismatchError`).
    DimensionMismatch = 2,
    /// `config_hash` of a compressed vector does not match the supplied
    /// `CodecConfig` (maps to Python `ConfigMismatchError`).
    ConfigMismatch = 3,
    /// Codebook bit width does not match the config (maps to Python
    /// `CodebookIncompatibleError`).
    CodebookIncompatible = 4,
    /// Duplicate `vector_id` on a corpus insert (maps to Python
    /// `DuplicateVectorError`).
    DuplicateVector = 5,
    /// I/O layer error — serialization / file / format issue.
    Io = 6,
    /// Caller passed a malformed argument (null where non-null required,
    /// out-of-range integer, wrong buffer size, etc.).
    InvalidArgument = 7,
    /// A panic crossed the `extern "C"` boundary. The wrapper captured
    /// the payload and set `message` accordingly. No UB.
    Panic = 98,
    /// An error kind the wrapper could not classify.
    Unknown = 99,
}

/// Out-pointer structure populated by every fallible `tq_*` entry point.
///
/// The caller owns `message` and MUST free it with [`tq_error_free`] or
/// [`tq_error_free_message`]. A fresh `TinyQuantError` may be zero-initialised
/// on the C side with `{0}`; Rust callers use [`TinyQuantError::empty`].
#[repr(C)]
#[derive(Debug)]
pub struct TinyQuantError {
    /// Discriminant. `Ok` means `message` is guaranteed null.
    pub kind: TinyQuantErrorKind,
    /// Owned NUL-terminated UTF-8 message; null when `kind == Ok`.
    pub message: *mut c_char,
}

impl TinyQuantError {
    /// Return a zero-initialised `TinyQuantError` suitable for passing to
    /// a fallible entry point.
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            kind: TinyQuantErrorKind::Ok,
            message: ptr::null_mut(),
        }
    }
}

impl Default for TinyQuantError {
    fn default() -> Self {
        Self::empty()
    }
}

// SAFETY: `TinyQuantError` carries a `*mut c_char` which is not
// automatically Send/Sync. The wrapper never aliases the pointer across
// threads; the C API is single-threaded per §Ownership rules.
// We intentionally do NOT implement Send/Sync.

/// Map a [`CodecError`] to `(kind, message)`.
#[allow(clippy::needless_pass_by_value)] // keeps callers terse; CodecError is Clone
fn codec_kind_and_message(err: CodecError) -> (TinyQuantErrorKind, String) {
    let message = err.to_string();
    let kind = match err {
        CodecError::DimensionMismatch { .. } => TinyQuantErrorKind::DimensionMismatch,
        CodecError::ConfigMismatch { .. } => TinyQuantErrorKind::ConfigMismatch,
        CodecError::CodebookIncompatible { .. } => TinyQuantErrorKind::CodebookIncompatible,
        // Every other variant (including future non_exhaustive additions)
        // is treated as a caller-driven invalid argument.
        _ => TinyQuantErrorKind::InvalidArgument,
    };
    (kind, message)
}

/// Map a [`CorpusError`] to `(kind, message)`.
#[allow(clippy::needless_pass_by_value)]
fn corpus_kind_and_message(err: CorpusError) -> (TinyQuantErrorKind, String) {
    let message = err.to_string();
    let kind = match err {
        CorpusError::Codec(inner) => return codec_kind_and_message(inner),
        CorpusError::DuplicateVectorId { .. } => TinyQuantErrorKind::DuplicateVector,
        CorpusError::DimensionMismatch { .. } => TinyQuantErrorKind::DimensionMismatch,
        // Catch-all for non_exhaustive additions.
        _ => TinyQuantErrorKind::InvalidArgument,
    };
    (kind, message)
}

#[allow(clippy::needless_pass_by_value)]
fn backend_kind_and_message(err: BackendError) -> (TinyQuantErrorKind, String) {
    let message = err.to_string();
    let kind = match err {
        BackendError::Empty | BackendError::InvalidTopK => TinyQuantErrorKind::InvalidArgument,
        BackendError::Adapter(_) => TinyQuantErrorKind::Unknown,
        _ => TinyQuantErrorKind::Unknown,
    };
    (kind, message)
}

/// Trait used by the ABI wrappers to convert any error-like value into
/// `(kind, message)` without a big `match` in every call site.
pub(crate) trait IntoAbiError {
    fn into_abi(self) -> (TinyQuantErrorKind, String);
}

impl IntoAbiError for CodecError {
    fn into_abi(self) -> (TinyQuantErrorKind, String) {
        codec_kind_and_message(self)
    }
}

impl IntoAbiError for CorpusError {
    fn into_abi(self) -> (TinyQuantErrorKind, String) {
        corpus_kind_and_message(self)
    }
}

impl IntoAbiError for BackendError {
    fn into_abi(self) -> (TinyQuantErrorKind, String) {
        backend_kind_and_message(self)
    }
}

impl IntoAbiError for tinyquant_io::errors::IoError {
    fn into_abi(self) -> (TinyQuantErrorKind, String) {
        (TinyQuantErrorKind::Io, self.to_string())
    }
}

/// Populate a caller-supplied `TinyQuantError` from any ABI-classifiable
/// error, returning the kind for the `extern "C" fn` to propagate.
///
/// If `out` is null, the message is dropped and only the kind is returned.
pub(crate) fn set_error<E: IntoAbiError>(out: *mut TinyQuantError, err: E) -> TinyQuantErrorKind {
    let (kind, message) = err.into_abi();
    write_error(out, kind, &message);
    kind
}

/// Populate a caller-supplied `TinyQuantError` with an [`InvalidArgument`]
/// error and a custom message.
///
/// [`InvalidArgument`]: TinyQuantErrorKind::InvalidArgument
pub(crate) fn set_invalid_argument(out: *mut TinyQuantError, message: &str) -> TinyQuantErrorKind {
    write_error(out, TinyQuantErrorKind::InvalidArgument, message);
    TinyQuantErrorKind::InvalidArgument
}

/// Populate a caller-supplied `TinyQuantError` with an [`InvalidHandle`]
/// error.
///
/// [`InvalidHandle`]: TinyQuantErrorKind::InvalidHandle
pub(crate) fn set_invalid_handle(out: *mut TinyQuantError) -> TinyQuantErrorKind {
    write_error(
        out,
        TinyQuantErrorKind::InvalidHandle,
        "handle pointer was null",
    );
    TinyQuantErrorKind::InvalidHandle
}

/// Populate a caller-supplied `TinyQuantError` from a `catch_unwind`
/// payload and return [`TinyQuantErrorKind::Panic`].
pub(crate) fn set_panic_error(
    out: *mut TinyQuantError,
    payload: &(dyn Any + Send),
) -> TinyQuantErrorKind {
    let message = extract_panic_message(payload);
    write_error(out, TinyQuantErrorKind::Panic, &message);
    TinyQuantErrorKind::Panic
}

/// Extract a human-readable message from a panic payload.
fn extract_panic_message(payload: &(dyn Any + Send)) -> String {
    payload.downcast_ref::<&'static str>().map_or_else(
        || {
            payload.downcast_ref::<String>().map_or_else(
                || format!("panic payload of type {:?}", payload.type_id()),
                Clone::clone,
            )
        },
        |s| (*s).to_owned(),
    )
}

fn write_error(out: *mut TinyQuantError, kind: TinyQuantErrorKind, message: &str) {
    if out.is_null() {
        return;
    }
    // Null-byte safety: CString::new fails if `message` contains NUL; fall
    // back to a sanitised copy so we always produce a valid C string.
    let cstr = CString::new(message.to_owned()).unwrap_or_else(|_| {
        let sanitised: String = message.chars().filter(|&c| c != '\0').collect();
        CString::new(sanitised).unwrap_or_else(|_| CString::new("<invalid>").unwrap_or_default())
    });
    let ptr = cstr.into_raw();
    // SAFETY: `out` verified non-null above; `TinyQuantError` is `#[repr(C)]`
    // with stable field offsets so direct write is well-defined.
    unsafe {
        (*out).kind = kind;
        // If a previous call left a message behind, free it to prevent
        // leaks across chained calls that reuse the same error struct.
        if !(*out).message.is_null() {
            let old = (*out).message;
            (*out).message = core::ptr::null_mut();
            drop(CString::from_raw(old));
        }
        (*out).message = ptr;
    }
}

/// Free the `message` pointer owned by a [`TinyQuantError`] and reset the
/// struct to `Ok`.
///
/// Idempotent — calling more than once on the same struct is safe.
/// Accepts a null `err` pointer as a no-op.
///
/// # Safety
///
/// - `err`, if non-null, must point to a `TinyQuantError` populated by a
///   TinyQuant entry point or zero-initialised by the caller.
/// - The `message` field must not have been freed by any allocator other
///   than this function.
#[no_mangle]
pub unsafe extern "C" fn tq_error_free(err: *mut TinyQuantError) {
    if err.is_null() {
        return;
    }
    unsafe {
        let msg = (*err).message;
        (*err).message = ptr::null_mut();
        (*err).kind = TinyQuantErrorKind::Ok;
        if !msg.is_null() {
            drop(CString::from_raw(msg));
        }
    }
}

/// Free a `message` pointer extracted from a [`TinyQuantError`].
///
/// Useful when the caller wants to extract the `message`, print it, then
/// release the allocation without touching the surrounding struct.
///
/// # Safety
///
/// `message`, if non-null, must have originated from a `TinyQuantError`
/// populated by a TinyQuant entry point (i.e. a Rust `CString::into_raw`
/// allocation).
#[no_mangle]
pub unsafe extern "C" fn tq_error_free_message(message: *mut c_char) {
    if message.is_null() {
        return;
    }
    unsafe {
        drop(CString::from_raw(message));
    }
}
