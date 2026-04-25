//! Verifies that panics raised inside an `extern "C" fn` are caught by
//! the crate's `catch_unwind` wrapper and reported as
//! [`TinyQuantErrorKind::Panic`] instead of unwinding into C.
//!
//! Uses the test-only probe `tq_test_panic`, which is gated behind
//! `cfg(any(test, feature = "panic-probe"))`. Integration tests compile
//! `tinyquant-sys` as an external crate, so its `cfg(test)` does not
//! fire — we therefore gate the whole body on the `panic-probe` feature
//! and emit a skip notice when it is off.
//!
//! Drive this file explicitly with:
//!
//! ```text
//! cargo test -p tinyquant-sys --features panic-probe --test abi_panic_crossing
//! ```
//!
//! See `docs/design/rust/ffi-and-bindings.md` §Binding 2 (panic
//! handling) and `docs/plans/rust/phase-22-pyo3-cabi-release.md` §C-ABI
//! test surface.

#![allow(unsafe_code, clippy::undocumented_unsafe_blocks)]

#[cfg(not(feature = "panic-probe"))]
#[test]
fn skipped_without_panic_probe_feature() {
    eprintln!(
        "abi_panic_crossing skipped: `tq_test_panic` is gated on \
         the `panic-probe` feature. Re-run with \
         `cargo test -p tinyquant-sys --features panic-probe \
         --test abi_panic_crossing` to exercise the probe."
    );
}

#[cfg(feature = "panic-probe")]
mod enabled {
    use core::ffi::CStr;
    use core::ptr;

    use tinyquant_sys::codec_abi::tq_test_panic;
    use tinyquant_sys::error::tq_error_free;
    use tinyquant_sys::{TinyQuantError, TinyQuantErrorKind};

    fn empty_error() -> TinyQuantError {
        TinyQuantError {
            kind: TinyQuantErrorKind::Ok,
            message: ptr::null_mut(),
        }
    }

    #[test]
    fn panic_inside_extern_c_is_caught_and_mapped() {
        let mut err = empty_error();
        let kind = unsafe { tq_test_panic(&mut err) };
        assert_eq!(kind, TinyQuantErrorKind::Panic);
        assert_eq!(err.kind, TinyQuantErrorKind::Panic);
        assert!(!err.message.is_null(), "panic must attach a message");
        let msg = unsafe { CStr::from_ptr(err.message) }
            .to_str()
            .expect("panic message is UTF-8");
        assert!(
            msg.contains("deliberate panic"),
            "panic message should mirror the probe payload; got: {msg}"
        );
        unsafe { tq_error_free(&mut err) };
        assert!(err.message.is_null(), "tq_error_free clears message");
        assert_eq!(err.kind, TinyQuantErrorKind::Ok);
    }

    #[test]
    fn repeated_panic_crossing_does_not_leak_kind() {
        for _ in 0..4 {
            let mut err = empty_error();
            let kind = unsafe { tq_test_panic(&mut err) };
            assert_eq!(kind, TinyQuantErrorKind::Panic);
            unsafe { tq_error_free(&mut err) };
        }
    }

    #[test]
    fn panic_crossing_with_null_err_pointer_is_safe() {
        let kind = unsafe { tq_test_panic(ptr::null_mut()) };
        assert_eq!(kind, TinyQuantErrorKind::Panic);
    }
}
