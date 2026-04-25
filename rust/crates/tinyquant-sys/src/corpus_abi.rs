//! Corpus layer of the TinyQuant C ABI.
//!
//! Exposes a minimal subset of [`tinyquant_core::corpus::Corpus`]:
//! construction, insertion, lookup, count, decompression, and teardown.
//! The signature conventions match [`crate::codec_abi`].
//!
//! See `docs/design/rust/ffi-and-bindings.md` §Binding 2 for the design
//! source of truth.
//!
//! # Why this module allows `unsafe_code`
//!
//! Same reasoning as [`crate::codec_abi`]: the module's entry points
//! receive raw pointers from C, transfer ownership via
//! `Box::into_raw`/`Box::from_raw`, and slice `*const f32` into Rust
//! slices. The `#![allow(unsafe_code)]` is scoped to this file.
#![allow(unsafe_code)]
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap
)]

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use core::ffi::CStr;
use core::panic::AssertUnwindSafe;
use core::ptr;
use core::slice;
use std::panic;

use tinyquant_core::codec::Codebook;
use tinyquant_core::corpus::{CompressionPolicy, Corpus};

use crate::error::{
    set_error, set_invalid_argument, set_invalid_handle, set_panic_error, TinyQuantError,
    TinyQuantErrorKind,
};
use crate::handle::{CodebookHandle, CodecConfigHandle, CorpusHandle};
use crate::internal::CodecConfigBox;

/// Compression-policy discriminator over the C ABI.
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TinyQuantCompressionPolicy {
    /// Lossy compression through the TinyQuant codec.
    Compress = 0,
    /// Store the FP32 vector verbatim (no compression).
    Passthrough = 1,
    /// Cast to FP16 without quantisation.
    Fp16 = 2,
}

impl From<TinyQuantCompressionPolicy> for CompressionPolicy {
    fn from(value: TinyQuantCompressionPolicy) -> Self {
        match value {
            TinyQuantCompressionPolicy::Compress => Self::Compress,
            TinyQuantCompressionPolicy::Passthrough => Self::Passthrough,
            TinyQuantCompressionPolicy::Fp16 => Self::Fp16,
        }
    }
}

/// Construct a new [`Corpus`] handle.
///
/// `corpus_id` is a borrowed NUL-terminated UTF-8 string; the corpus
/// copies it internally.
///
/// # Safety
///
/// - `corpus_id` must be a valid pointer to a NUL-terminated UTF-8
///   string or null (treated as `"corpus"`).
/// - `config`, `codebook` must be live handles.
/// - `out`, `err` must satisfy the usual contracts.
#[no_mangle]
pub unsafe extern "C" fn tq_corpus_new(
    corpus_id: *const core::ffi::c_char,
    config: *const CodecConfigHandle,
    codebook: *const CodebookHandle,
    policy: TinyQuantCompressionPolicy,
    out: *mut *mut CorpusHandle,
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
        let id: Arc<str> = if corpus_id.is_null() {
            Arc::from("corpus")
        } else {
            let cstr = unsafe { CStr::from_ptr(corpus_id) };
            match cstr.to_str() {
                Ok(s) => Arc::from(s),
                Err(_) => return set_invalid_argument(err, "corpus_id is not valid UTF-8"),
            }
        };
        let cfg = unsafe { &(*config.cast::<CodecConfigBox>()).inner };
        let cb = unsafe { &*codebook.cast::<Codebook>() };
        let corpus = Corpus::new(id, cfg.clone(), cb.clone(), policy.into(), BTreeMap::new());
        let raw = Box::into_raw(Box::new(corpus)).cast::<CorpusHandle>();
        unsafe { *out = raw };
        TinyQuantErrorKind::Ok
    }));
    match result {
        Ok(kind) => kind,
        Err(payload) => set_panic_error(err, &*payload),
    }
}

/// Insert a vector into the corpus with a given NUL-terminated id.
///
/// # Safety
///
/// - `handle` must be a live corpus handle.
/// - `vector_id` must be a valid NUL-terminated UTF-8 string.
/// - `vector` must point to at least `vector_len` `f32`s.
#[no_mangle]
pub unsafe extern "C" fn tq_corpus_insert(
    handle: *mut CorpusHandle,
    vector_id: *const core::ffi::c_char,
    vector: *const f32,
    vector_len: usize,
    timestamp: i64,
    err: *mut TinyQuantError,
) -> TinyQuantErrorKind {
    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        if handle.is_null() {
            return set_invalid_handle(err);
        }
        if vector_id.is_null() {
            return set_invalid_argument(err, "vector_id pointer is null");
        }
        if vector.is_null() {
            return set_invalid_argument(err, "vector pointer is null");
        }
        let cstr = unsafe { CStr::from_ptr(vector_id) };
        let Ok(id_str) = cstr.to_str() else {
            return set_invalid_argument(err, "vector_id is not valid UTF-8");
        };
        let id: Arc<str> = Arc::from(id_str);
        let vec_slice = unsafe { slice::from_raw_parts(vector, vector_len) };
        let corpus = unsafe { &mut *handle.cast::<Corpus>() };
        match corpus.insert(id, vec_slice, None, timestamp) {
            Ok(()) => TinyQuantErrorKind::Ok,
            Err(e) => set_error(err, e),
        }
    }));
    match result {
        Ok(kind) => kind,
        Err(payload) => set_panic_error(err, &*payload),
    }
}

/// Return the number of vectors in the corpus. Returns 0 on null.
///
/// # Safety
///
/// `handle` must be null or a live corpus handle.
#[no_mangle]
pub unsafe extern "C" fn tq_corpus_vector_count(handle: *const CorpusHandle) -> usize {
    if handle.is_null() {
        return 0;
    }
    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        let corpus = unsafe { &*handle.cast::<Corpus>() };
        corpus.vector_count()
    }));
    result.unwrap_or(0)
}

/// Return `true` iff `vector_id` is present in the corpus.
///
/// # Safety
///
/// `handle` must be null or a live corpus handle. `vector_id` must be a
/// NUL-terminated UTF-8 string.
#[no_mangle]
pub unsafe extern "C" fn tq_corpus_contains(
    handle: *const CorpusHandle,
    vector_id: *const core::ffi::c_char,
) -> bool {
    if handle.is_null() || vector_id.is_null() {
        return false;
    }
    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        let cstr = unsafe { CStr::from_ptr(vector_id) };
        let Ok(id_str) = cstr.to_str() else {
            return false;
        };
        let id: Arc<str> = Arc::from(id_str);
        let corpus = unsafe { &*handle.cast::<Corpus>() };
        corpus.contains(&id)
    }));
    result.unwrap_or(false)
}

/// Free a corpus handle. Accepts null.
///
/// # Safety
///
/// `handle`, if non-null, must be a pointer previously returned by
/// [`tq_corpus_new`].
#[no_mangle]
pub unsafe extern "C" fn tq_corpus_free(handle: *mut CorpusHandle) {
    if handle.is_null() {
        return;
    }
    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        let boxed = unsafe { Box::from_raw(handle.cast::<Corpus>()) };
        drop(boxed);
    }));
    let _ = result;
}
