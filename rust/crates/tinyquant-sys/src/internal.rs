//! Crate-private layout types shared between [`crate::codec_abi`] and
//! [`crate::corpus_abi`].
//!
//! These types are pointer-cast to from opaque handles declared in
//! [`crate::handle`]. Keeping the definitions in one place guarantees a
//! single ground-truth layout: `codec_abi` performs
//! `Box::into_raw::<CodecConfigBox>()`, and `corpus_abi` dereferences via
//! the identical `CodecConfigBox` type — so there is no way for the two
//! modules to drift out of sync on field offsets.
//!
//! The module itself contains no `unsafe` code; only plain value types.

use alloc::ffi::CString;

use tinyquant_core::codec::CodecConfig;

/// Internal wrapper that pairs a [`CodecConfig`] with its cached hex
/// `config_hash` as a [`CString`] so we can hand a borrowed
/// `*const c_char` back to C without allocating per call.
pub(crate) struct CodecConfigBox {
    pub(crate) inner: CodecConfig,
    pub(crate) hash_cstring: CString,
}

impl CodecConfigBox {
    pub(crate) fn new(inner: CodecConfig) -> Self {
        let hash = inner.config_hash().as_ref();
        // The hash is ASCII hex digits only — `CString::new` cannot fail.
        let hash_cstring = CString::new(hash).unwrap_or_default();
        Self {
            inner,
            hash_cstring,
        }
    }
}
