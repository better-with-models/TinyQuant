//! Rust-side smoke test for the `tinyquant-sys` C ABI.
//!
//! Exercises every `tq_*` entry point through its Rust re-export and
//! asserts the ownership, error, and out-parameter semantics documented
//! in `docs/design/rust/ffi-and-bindings.md` §Binding 2.

#![allow(unsafe_code, clippy::undocumented_unsafe_blocks)]

use core::ffi::c_char;

use tinyquant_sys::codec_abi::{
    tq_codec_config_bit_width, tq_codec_config_dimension, tq_codec_config_free,
    tq_codec_config_hash, tq_codec_config_new, tq_codec_config_residual_enabled,
    tq_codec_config_seed, tq_version,
};
use tinyquant_sys::error::{tq_error_free, tq_error_free_message};
use tinyquant_sys::{CodecConfigHandle, TinyQuantError, TinyQuantErrorKind};

fn empty_error() -> TinyQuantError {
    TinyQuantError {
        kind: TinyQuantErrorKind::Ok,
        message: core::ptr::null_mut(),
    }
}

#[test]
fn config_handle_round_trip() {
    let mut handle: *mut CodecConfigHandle = core::ptr::null_mut();
    let mut err = empty_error();
    let kind = unsafe { tq_codec_config_new(4, 42, 768, true, &mut handle, &mut err) };
    assert_eq!(kind, TinyQuantErrorKind::Ok);
    assert!(!handle.is_null());
    unsafe {
        assert_eq!(tq_codec_config_bit_width(handle), 4);
        assert_eq!(tq_codec_config_seed(handle), 42);
        assert_eq!(tq_codec_config_dimension(handle), 768);
        assert!(tq_codec_config_residual_enabled(handle));
        let hash = tq_codec_config_hash(handle);
        assert!(!hash.is_null());
        // 64-char hex digest plus NUL terminator.
        let s = core::ffi::CStr::from_ptr(hash).to_str().unwrap();
        assert_eq!(s.len(), 64);
        tq_codec_config_free(handle);
    }
    // tq_error_free must be safe to call when message is null.
    unsafe { tq_error_free(&mut err) };
}

#[test]
fn config_invalid_bit_width_returns_error() {
    let mut handle: *mut CodecConfigHandle = core::ptr::null_mut();
    let mut err = empty_error();
    let kind = unsafe { tq_codec_config_new(3, 0, 8, true, &mut handle, &mut err) };
    assert_ne!(kind, TinyQuantErrorKind::Ok);
    assert!(handle.is_null());
    assert!(!err.message.is_null());
    unsafe { tq_error_free(&mut err) };
    // Calling free again (message now null) is fine.
    unsafe { tq_error_free(&mut err) };
}

#[test]
fn config_null_out_ptr_is_invalid_argument() {
    let mut err = empty_error();
    let kind = unsafe { tq_codec_config_new(4, 0, 8, true, core::ptr::null_mut(), &mut err) };
    assert_eq!(kind, TinyQuantErrorKind::InvalidArgument);
    unsafe { tq_error_free(&mut err) };
}

#[test]
fn free_null_config_is_noop() {
    unsafe { tq_codec_config_free(core::ptr::null_mut()) };
}

#[test]
fn version_string_is_sane() {
    let ptr: *const c_char = tq_version();
    assert!(!ptr.is_null());
    let s = unsafe { core::ffi::CStr::from_ptr(ptr).to_str().unwrap() };
    assert_eq!(s, env!("CARGO_PKG_VERSION"));
}

#[test]
fn error_free_message_only_is_noop_on_null() {
    unsafe { tq_error_free_message(core::ptr::null_mut()) };
}
