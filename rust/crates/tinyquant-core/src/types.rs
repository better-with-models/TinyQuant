//! Shared primitive type aliases mirroring `tinyquant_cpu._types`.
//!
//! See `docs/design/rust/type-mapping.md` for the complete mapping
//! table and rationale.

use alloc::sync::Arc;

/// Stable string identity for a vector inside a [`Corpus`].
///
/// `Arc<str>` gives Python-like shareability (O(1) clones) without
/// allocating a new buffer on each clone. Compare to Python `str`.
pub type VectorId = Arc<str>;

/// SHA-256 hex digest that uniquely identifies a [`CodecConfig`].
///
/// Produced by [`CodecConfig::config_hash`] and stored in every
/// [`CompressedVector`] for config-mismatch detection.
pub type ConfigHash = Arc<str>;

/// Stable string identity for a [`Corpus`].
pub type CorpusId = Arc<str>;

/// Owned, heap-allocated 1-D FP32 vector.
///
/// Functions that only need to read a vector accept [`VectorSlice`]
/// instead to avoid forcing a clone.
// Use the full path to avoid a dead `use alloc::vec::Vec` import that
// would trip `#![deny(warnings)]` since the alias itself is not used
// as a value in this file.
pub type Vector = alloc::vec::Vec<f32>;

/// Borrowed immutable view of a 1-D FP32 vector.
///
/// All codec read-path functions accept this instead of `&Vector`
/// to be compatible with any contiguous FP32 backing store.
pub type VectorSlice<'a> = &'a [f32];

/// Nanoseconds since the Unix epoch, stored as a signed 64-bit integer.
///
/// Kept as `i64` to stay `no_std` (no `std::time`). Crates with `std`
/// access (`tinyquant-io`, `tinyquant-py`) convert to and from
/// `std::time::SystemTime` at the boundary.
pub type Timestamp = i64;
