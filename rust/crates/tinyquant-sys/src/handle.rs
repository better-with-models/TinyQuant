//! Opaque handle types exposed by the TinyQuant C ABI.
//!
//! Every public handle is declared as
//!
//! ```ignore
//! #[repr(C)] pub struct FooHandle { _private: [u8; 0] }
//! ```
//!
//! so that C consumers can only hold pointers to `FooHandle*` and never
//! look at or modify its fields. The actual Rust objects live behind
//! these pointers via `Box::into_raw` / `Box::from_raw`; see
//! `docs/design/rust/ffi-and-bindings.md` §Binding 2.
//!
//! This module is safe — it only declares the opaque types. Handle
//! construction / destruction is performed by [`crate::codec_abi`] and
//! [`crate::corpus_abi`], which own the narrow `#[allow(unsafe_code)]`
//! gates.

/// Opaque handle for `tinyquant_core::codec::CodecConfig`.
///
/// The real object is a `Box<CodecConfig>`; pointers are minted with
/// `Box::into_raw`. Never dereferenced in this module.
#[repr(C)]
pub struct CodecConfigHandle {
    _private: [u8; 0],
}

/// Opaque handle for `tinyquant_core::codec::Codebook`.
#[repr(C)]
pub struct CodebookHandle {
    _private: [u8; 0],
}

/// Opaque handle for `tinyquant_core::codec::CompressedVector`.
#[repr(C)]
pub struct CompressedVectorHandle {
    _private: [u8; 0],
}

/// Opaque handle for `tinyquant_core::corpus::Corpus`.
#[repr(C)]
pub struct CorpusHandle {
    _private: [u8; 0],
}

/// Opaque handle for a byte buffer owned by Rust (returned from
/// `tq_compressed_vector_to_bytes`).
///
/// The caller frees it with `tq_bytes_free(ptr, len)`.
#[repr(C)]
pub struct ByteBufferHandle {
    _private: [u8; 0],
}
