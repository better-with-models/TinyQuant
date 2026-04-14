//! JavaScript/TypeScript bindings for TinyQuant (napi-rs).
//!
//! Phase 25.3: codec + corpus + backend surface. See
//! `docs/plans/rust/phase-25-typescript-npm-package.md` §Steps.
//!
//! This crate is intentionally thin: all algorithmic work is
//! delegated to `tinyquant-core` (and `tinyquant-bruteforce` for the
//! search backend). The napi-rs layer only handles type conversion
//! at the JS/Rust boundary.
//!
//! This crate is NOT `#![no_std]` because napi-rs v2's runtime relies
//! on `std` (thread locals, `String` owners in `Error::new`, etc.);
//! we still prefer `alloc::*` types in the codec module for symmetry
//! with `tinyquant-core` and to make a future `no_std` bridge less
//! intrusive.

use napi_derive::napi;

pub mod backend;
pub mod buffers;
pub mod codec;
pub mod corpus;
pub mod errors;

pub use backend::{BruteForceBackend, SearchResult};
pub use codec::{
    compress, decompress, Codebook, Codec, CodecConfig, CodecConfigOpts, CompressedVector,
    RotationMatrix,
};
pub use corpus::{CompressionPolicy, Corpus, VectorEntry};

/// Package version, forwarded from the workspace Cargo.toml.
#[napi]
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
