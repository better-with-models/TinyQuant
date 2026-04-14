//! Handle-lifetime tests for the `tinyquant-sys` C ABI.
//!
//! Covers the null / already-freed / wrong-handle-kind paths documented
//! in `docs/design/rust/ffi-and-bindings.md` §Binding 2. The crate
//! promises:
//!
//! - Every `*_free` entry point is a no-op on null.
//! - Every query accessor (`*_bit_width`, `*_dimension`, …) returns a
//!   sentinel (`0` / `false` / `null`) on null input, never panicking.
//! - Any other entry point that receives a null *handle* returns
//!   [`TinyQuantErrorKind::InvalidHandle`] without touching user memory.
//! - A null `out` pointer on a constructor is
//!   [`TinyQuantErrorKind::InvalidArgument`] — a caller bug that must
//!   not allocate.
//!
//! We do NOT test double-free: the contract is "must be called exactly
//! once per successful *_new", and verifying it would require
//! instrumenting the heap.

#![allow(unsafe_code, clippy::undocumented_unsafe_blocks)]

use core::ptr;

use tinyquant_sys::codec_abi::{
    tq_bytes_free, tq_codebook_bit_width, tq_codebook_free, tq_codec_compress,
    tq_codec_config_bit_width, tq_codec_config_dimension, tq_codec_config_free,
    tq_codec_config_hash, tq_codec_config_new, tq_codec_config_residual_enabled,
    tq_codec_config_seed, tq_codec_decompress, tq_compressed_vector_bit_width,
    tq_compressed_vector_dimension, tq_compressed_vector_free, tq_compressed_vector_from_bytes,
    tq_compressed_vector_to_bytes,
};
use tinyquant_sys::corpus_abi::{
    tq_corpus_contains, tq_corpus_free, tq_corpus_insert, tq_corpus_new, tq_corpus_vector_count,
    TinyQuantCompressionPolicy,
};
use tinyquant_sys::error::tq_error_free;
use tinyquant_sys::{
    CodebookHandle, CodecConfigHandle, CompressedVectorHandle, CorpusHandle, TinyQuantError,
    TinyQuantErrorKind,
};

fn empty_error() -> TinyQuantError {
    TinyQuantError {
        kind: TinyQuantErrorKind::Ok,
        message: ptr::null_mut(),
    }
}

// ── *_free on null ────────────────────────────────────────────────────

#[test]
fn all_free_entry_points_accept_null() {
    unsafe {
        tq_codec_config_free(ptr::null_mut());
        tq_codebook_free(ptr::null_mut());
        tq_compressed_vector_free(ptr::null_mut());
        tq_corpus_free(ptr::null_mut());
        tq_bytes_free(ptr::null_mut(), 0);
        tq_bytes_free(ptr::null_mut(), 42);
    }
}

// ── Accessors on null return sentinels, never panic ───────────────────

#[test]
fn accessors_return_sentinels_on_null() {
    unsafe {
        assert_eq!(tq_codec_config_bit_width(ptr::null()), 0);
        assert_eq!(tq_codec_config_seed(ptr::null()), 0);
        assert_eq!(tq_codec_config_dimension(ptr::null()), 0);
        assert!(!tq_codec_config_residual_enabled(ptr::null()));
        assert!(tq_codec_config_hash(ptr::null()).is_null());

        assert_eq!(tq_codebook_bit_width(ptr::null()), 0);

        assert_eq!(tq_compressed_vector_bit_width(ptr::null()), 0);
        assert_eq!(tq_compressed_vector_dimension(ptr::null()), 0);

        assert_eq!(tq_corpus_vector_count(ptr::null()), 0);
        assert!(!tq_corpus_contains(ptr::null(), b"x\0".as_ptr().cast()));
    }
}

// ── Null handle on fallible entry point returns InvalidHandle ─────────

#[test]
fn codebook_train_null_config_is_invalid_handle() {
    let mut out: *mut CodebookHandle = ptr::null_mut();
    let mut err = empty_error();
    let values = [0.0_f32; 8];
    let kind = unsafe {
        tinyquant_sys::codec_abi::tq_codebook_train(
            values.as_ptr(),
            1,
            8,
            ptr::null(),
            &mut out,
            &mut err,
        )
    };
    assert_eq!(kind, TinyQuantErrorKind::InvalidHandle);
    assert!(out.is_null());
    unsafe { tq_error_free(&mut err) };
}

#[test]
fn codec_compress_null_handles_are_invalid_handle() {
    let mut out: *mut CompressedVectorHandle = ptr::null_mut();
    let mut err = empty_error();
    let vector = [0.0_f32; 8];
    let kind = unsafe {
        tq_codec_compress(
            ptr::null(),
            ptr::null(),
            vector.as_ptr(),
            8,
            &mut out,
            &mut err,
        )
    };
    assert_eq!(kind, TinyQuantErrorKind::InvalidHandle);
    assert!(out.is_null());
    unsafe { tq_error_free(&mut err) };
}

#[test]
fn codec_decompress_null_handles_are_invalid_handle() {
    let mut err = empty_error();
    let mut out = [0.0_f32; 8];
    let kind = unsafe {
        tq_codec_decompress(
            ptr::null(),
            ptr::null(),
            ptr::null(),
            out.as_mut_ptr(),
            out.len(),
            &mut err,
        )
    };
    assert_eq!(kind, TinyQuantErrorKind::InvalidHandle);
    unsafe { tq_error_free(&mut err) };
}

#[test]
fn compressed_vector_from_bytes_null_data_with_nonzero_len_is_invalid_argument() {
    let mut out: *mut CompressedVectorHandle = ptr::null_mut();
    let mut err = empty_error();
    let kind = unsafe {
        // A null pointer with a non-zero length is unambiguously a
        // caller bug; the ABI must reject it before touching the buffer.
        tq_compressed_vector_from_bytes(ptr::null(), 16, &mut out, &mut err)
    };
    assert_eq!(kind, TinyQuantErrorKind::InvalidArgument);
    assert!(out.is_null());
    unsafe { tq_error_free(&mut err) };
}

#[test]
fn compressed_vector_from_bytes_empty_input_is_io_error() {
    let mut out: *mut CompressedVectorHandle = ptr::null_mut();
    let mut err = empty_error();
    let kind = unsafe {
        // Empty but well-formed input (null ptr + 0 len) is a valid
        // call shape; the serde layer then rejects it as malformed.
        tq_compressed_vector_from_bytes(ptr::null(), 0, &mut out, &mut err)
    };
    assert_eq!(kind, TinyQuantErrorKind::Io);
    assert!(out.is_null());
    unsafe { tq_error_free(&mut err) };
}

#[test]
fn compressed_vector_to_bytes_null_handle_is_invalid_handle() {
    let mut bytes: *mut u8 = ptr::null_mut();
    let mut len: usize = 0;
    let mut err = empty_error();
    let kind =
        unsafe { tq_compressed_vector_to_bytes(ptr::null(), &mut bytes, &mut len, &mut err) };
    assert_eq!(kind, TinyQuantErrorKind::InvalidHandle);
    assert!(bytes.is_null());
    unsafe { tq_error_free(&mut err) };
}

#[test]
fn corpus_new_null_handles_are_invalid_handle() {
    let mut out: *mut CorpusHandle = ptr::null_mut();
    let mut err = empty_error();
    let kind = unsafe {
        tq_corpus_new(
            ptr::null(),
            ptr::null(),
            ptr::null(),
            TinyQuantCompressionPolicy::Compress,
            &mut out,
            &mut err,
        )
    };
    assert_eq!(kind, TinyQuantErrorKind::InvalidHandle);
    assert!(out.is_null());
    unsafe { tq_error_free(&mut err) };
}

#[test]
fn corpus_insert_null_handle_is_invalid_handle() {
    let mut err = empty_error();
    let vector = [0.0_f32; 8];
    let kind = unsafe {
        tq_corpus_insert(
            ptr::null_mut(),
            b"id\0".as_ptr().cast(),
            vector.as_ptr(),
            8,
            0,
            &mut err,
        )
    };
    assert_eq!(kind, TinyQuantErrorKind::InvalidHandle);
    unsafe { tq_error_free(&mut err) };
}

// ── Null `out` constructor → InvalidArgument, no leak ─────────────────

#[test]
fn codec_config_new_null_out_is_invalid_argument() {
    let mut err = empty_error();
    let kind = unsafe { tq_codec_config_new(4, 0, 8, true, ptr::null_mut(), &mut err) };
    assert_eq!(kind, TinyQuantErrorKind::InvalidArgument);
    unsafe { tq_error_free(&mut err) };
}

// ── Live-handle round trip including free ─────────────────────────────
//
// A dedicated "round-trip" test to catch the most basic lifecycle
// regression: construct → free → nothing crashes.
#[test]
fn live_handle_round_trip_frees_cleanly() {
    let mut h: *mut CodecConfigHandle = ptr::null_mut();
    let mut err = empty_error();
    let kind = unsafe { tq_codec_config_new(4, 7, 16, true, &mut h, &mut err) };
    assert_eq!(kind, TinyQuantErrorKind::Ok);
    assert!(!h.is_null());
    unsafe { tq_codec_config_free(h) };
}
