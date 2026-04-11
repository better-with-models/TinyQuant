//! `VectorEntry` entity — one compressed vector stored in a [`Corpus`](super::Corpus).
//!
//! Identity is determined solely by [`VectorId`]; equality and hashing ignore
//! the compressed payload and metadata. This matches Python's behaviour where
//! `dict` keys are the vector identifiers.

use alloc::{collections::BTreeMap, string::String};

use crate::codec::CompressedVector;
use crate::corpus::entry_meta_value::EntryMetaValue;
use crate::types::{Timestamp, VectorId};

/// A single compressed vector together with its insertion timestamp and
/// arbitrary key-value metadata.
///
/// # Identity
///
/// `PartialEq`, `Eq`, and `Hash` are implemented **by `vector_id` only**.
/// Two entries with the same id but different payloads compare as equal.
/// This lets callers use `VectorEntry` as a set element or map key.
#[derive(Clone, Debug)]
pub struct VectorEntry {
    vector_id: VectorId,
    compressed: CompressedVector,
    inserted_at: Timestamp,
    metadata: BTreeMap<String, EntryMetaValue>,
}

impl VectorEntry {
    /// Construct a new `VectorEntry`.
    ///
    /// `inserted_at` should be a nanosecond Unix timestamp from the caller's
    /// time source.  `metadata` may be empty.
    #[must_use]
    pub const fn new(
        vector_id: VectorId,
        compressed: CompressedVector,
        inserted_at: Timestamp,
        metadata: BTreeMap<String, EntryMetaValue>,
    ) -> Self {
        Self {
            vector_id,
            compressed,
            inserted_at,
            metadata,
        }
    }

    /// The stable string identifier for this vector.
    #[must_use]
    pub const fn vector_id(&self) -> &VectorId {
        &self.vector_id
    }

    /// Reference to the compressed payload.
    #[must_use]
    pub const fn compressed(&self) -> &CompressedVector {
        &self.compressed
    }

    /// Nanosecond Unix timestamp recorded at insertion time.
    #[must_use]
    pub const fn inserted_at(&self) -> Timestamp {
        self.inserted_at
    }

    /// Read-only view of the metadata map.
    #[must_use]
    pub const fn metadata(&self) -> &BTreeMap<String, EntryMetaValue> {
        &self.metadata
    }

    /// Mutable access to the metadata map.
    pub fn metadata_mut(&mut self) -> &mut BTreeMap<String, EntryMetaValue> {
        &mut self.metadata
    }

    /// Convenience delegate: config hash of the underlying compressed vector.
    #[must_use]
    pub const fn config_hash(&self) -> &crate::types::ConfigHash {
        self.compressed.config_hash()
    }

    /// Convenience delegate: number of dimensions.
    #[must_use]
    pub const fn dimension(&self) -> u32 {
        self.compressed.dimension()
    }

    /// Convenience delegate: whether a residual buffer is present.
    #[must_use]
    pub fn has_residual(&self) -> bool {
        self.compressed.has_residual()
    }
}

impl PartialEq for VectorEntry {
    /// Equality by `vector_id` only — payload and metadata are ignored.
    fn eq(&self, other: &Self) -> bool {
        self.vector_id == other.vector_id
    }
}

impl Eq for VectorEntry {}

impl core::hash::Hash for VectorEntry {
    /// Hash by `vector_id` only — must be consistent with `PartialEq`.
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.vector_id.hash(state);
    }
}
