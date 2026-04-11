//! Compression policy enum and storage tag for the [`Corpus`](super::Corpus) aggregate.
//!
//! Three mutually exclusive policies control how a [`Corpus`] stores embedding
//! vectors:
//!
//! | Policy | Transform | `StorageTag` | Typical compression ratio |
//! |---|---|---|---|
//! | `Compress` | Full codec pipeline (rotate → quantize → optional residual) | `U8` | 4–8× |
//! | `Passthrough` | Raw f32 bytes, no transformation | `F32` | 1× |
//! | `Fp16` | Each f32 cast to f16 little-endian | `F16` | 2× |
//!
//! The policy is set at corpus construction time and is **immutable** after
//! the first `insert` call. Attempting to change it raises
//! [`CorpusError::PolicyImmutable`](crate::errors::CorpusError::PolicyImmutable).

/// Determines how vectors are stored in a [`Corpus`](super::Corpus).
///
/// The policy is set at construction and cannot be changed after the first
/// [`insert`](super::Corpus::insert) call.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CompressionPolicy {
    /// Full codec pipeline: rotate → scalar quantize → optional FP16 residual.
    ///
    /// Requires a trained [`Codebook`](crate::codec::Codebook) and a
    /// [`CodecConfig`](crate::codec::CodecConfig). Produces `StorageTag::U8`
    /// output.
    Compress,

    /// Store raw f32 bytes with no transformation (1× compression ratio).
    ///
    /// The four bytes of each `f32` are stored in little-endian order inside
    /// the `indices` field of a [`CompressedVector`](crate::codec::CompressedVector).
    /// Produces `StorageTag::F32` output.
    Passthrough,

    /// Cast each `f32` to a 16-bit float and store 2 bytes per dimension
    /// in little-endian order (2× compression ratio).
    ///
    /// Precision loss is bounded by the f16 range; `max |error|` is
    /// typically below `1e-2` for normalised embedding values.
    /// Produces `StorageTag::F16` output.
    Fp16,
}

impl CompressionPolicy {
    /// Returns `true` for policies that run the full codec pipeline.
    ///
    /// Only [`CompressionPolicy::Compress`] requires a trained codebook;
    /// the other policies bypass the codec entirely.
    #[must_use]
    pub const fn requires_codec(&self) -> bool {
        matches!(self, Self::Compress)
    }

    /// Returns the [`StorageTag`] that describes the byte layout used by this policy.
    #[must_use]
    pub const fn storage_tag(&self) -> StorageTag {
        match self {
            Self::Compress => StorageTag::U8,
            Self::Passthrough => StorageTag::F32,
            Self::Fp16 => StorageTag::F16,
        }
    }
}

/// Describes the physical byte layout of stored vector data.
///
/// Returned by [`CompressionPolicy::storage_tag`]. Used by decompression
/// routines to select the correct byte-interpretation strategy without
/// inspecting the policy directly.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StorageTag {
    /// One byte per dimension (codec quantisation index, 2/4/8-bit packed in 1 byte).
    U8,
    /// Four bytes per dimension, raw `f32` little-endian.
    F32,
    /// Two bytes per dimension, `f16` little-endian.
    F16,
}
