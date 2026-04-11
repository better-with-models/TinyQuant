//! [`Corpus`] aggregate root — manages a collection of compressed embedding vectors.
//!
//! The `Corpus` is the primary domain object in `TinyQuant`'s corpus layer.  It
//! owns an insertion-ordered map of [`VectorEntry`] values, enforces the active
//! [`CompressionPolicy`], buffers domain events, and exposes a clean API for
//! insertion, decompression, and event draining.
//!
//! ## Thread safety
//!
//! `Corpus` is not `Sync`.  Callers that need concurrent access must wrap it in
//! an external lock.
//!
//! ## Passthrough and Fp16 policies
//!
//! For `Passthrough`, each f32 dimension is stored as 4 little-endian bytes
//! in the `indices` field of a [`CompressedVector`] with `bit_width = 8`.
//! For `Fp16`, each f32 is cast to f16 and stored as 2 little-endian bytes
//! (2 dimensions packed per byte pair, with `bit_width = 8` to keep the
//! `CompressedVector` invariant satisfied).

use alloc::{
    boxed::Box,
    collections::BTreeMap,
    string::{String, ToString},
    sync::Arc,
    vec,
    vec::Vec,
};

use half::f16;

use crate::codec::{Codebook, Codec, CodecConfig, CompressedVector};
use crate::corpus::compression_policy::CompressionPolicy;
use crate::corpus::entry_meta_value::EntryMetaValue;
use crate::corpus::events::CorpusEvent;
use crate::corpus::vector_entry::VectorEntry;
use crate::errors::CorpusError;
use crate::types::{CorpusId, Timestamp, VectorId};

// Use VectorIdMap for both std and no_std builds in Phase 18.
// IndexMap integration is deferred to Phase 21+ per the plan recommendation.
use crate::corpus::vector_id_map::VectorIdMap;

/// Accounting summary returned by [`Corpus::insert_batch`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BatchReport {
    /// Number of vectors successfully inserted.
    ///
    /// In strict (all-or-nothing) mode this is either `0` (on `Err`) or
    /// `vectors.len()` (on `Ok`).
    pub inserted: usize,
    /// Number of vectors skipped due to soft failure.
    ///
    /// Always `0` in strict mode; non-zero only in lenient mode (Phase 22+).
    pub skipped: usize,
    /// First error encountered, with its zero-based batch index.
    ///
    /// `None` on strict-mode success; `Some` in lenient mode when any vector
    /// was rejected.
    pub first_error: Option<(usize, CorpusError)>,
}

/// Aggregate root that manages a collection of compressed embedding vectors.
///
/// # Domain contracts
///
/// - `CompressionPolicy` is immutable after the first successful `insert`.
/// - Domain events are buffered and drained atomically via [`drain_events`](Self::drain_events).
/// - `insert_batch` is all-or-nothing: on error the corpus is not modified.
///
/// # Usage
///
/// ```ignore
/// let mut corpus = Corpus::new(
///     Arc::from("my-corpus"),
///     config,
///     codebook,
///     CompressionPolicy::Compress,
///     BTreeMap::new(),
/// );
/// corpus.insert(Arc::from("v1"), &vector, None)?;
/// let events = corpus.drain_events();
/// ```
pub struct Corpus {
    corpus_id: CorpusId,
    config: CodecConfig,
    codebook: Codebook,
    compression_policy: CompressionPolicy,
    vectors: VectorIdMap<VectorEntry>,
    pending_events: Vec<CorpusEvent>,
    metadata: BTreeMap<String, EntryMetaValue>,
}

impl Corpus {
    /// Construct a new `Corpus` and emit a [`CorpusEvent::Created`] event.
    ///
    /// `timestamp` must be supplied by the caller (nanoseconds since Unix
    /// epoch) to keep this crate `no_std` (no `std::time`).
    #[must_use]
    pub fn new(
        corpus_id: CorpusId,
        config: CodecConfig,
        codebook: Codebook,
        compression_policy: CompressionPolicy,
        metadata: BTreeMap<String, EntryMetaValue>,
    ) -> Self {
        Self::new_at(corpus_id, config, codebook, compression_policy, metadata, 0)
    }

    /// Construct a new `Corpus` with an explicit creation timestamp.
    ///
    /// Use this variant in tests or callers that supply their own time source.
    #[must_use]
    pub fn new_at(
        corpus_id: CorpusId,
        config: CodecConfig,
        codebook: Codebook,
        compression_policy: CompressionPolicy,
        metadata: BTreeMap<String, EntryMetaValue>,
        timestamp: Timestamp,
    ) -> Self {
        let event = CorpusEvent::Created {
            corpus_id: Arc::clone(&corpus_id),
            codec_config: config.clone(),
            compression_policy,
            timestamp,
        };
        Self {
            corpus_id,
            config,
            codebook,
            compression_policy,
            vectors: VectorIdMap::new(),
            pending_events: vec![event],
            metadata,
        }
    }

    // ── Accessors ─────────────────────────────────────────────────────────────

    /// The stable string identifier for this corpus.
    #[must_use]
    pub const fn corpus_id(&self) -> &CorpusId {
        &self.corpus_id
    }

    /// The codec configuration used at construction time.
    #[must_use]
    pub const fn config(&self) -> &CodecConfig {
        &self.config
    }

    /// The active compression policy.
    #[must_use]
    pub const fn compression_policy(&self) -> CompressionPolicy {
        self.compression_policy
    }

    /// Number of vectors currently stored in the corpus.
    #[must_use]
    pub fn vector_count(&self) -> usize {
        self.vectors.len()
    }

    /// Returns `true` when the corpus contains no vectors.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.vectors.is_empty()
    }

    /// Returns `true` if a vector with the given id exists.
    #[must_use]
    pub fn contains(&self, id: &VectorId) -> bool {
        self.vectors.contains_key(id.as_ref())
    }

    /// Iterate over `(VectorId, VectorEntry)` pairs in insertion order.
    pub fn iter(&self) -> impl Iterator<Item = (&VectorId, &VectorEntry)> {
        self.vectors.iter()
    }

    /// Read-only access to the corpus-level metadata.
    #[must_use]
    pub const fn metadata(&self) -> &BTreeMap<String, EntryMetaValue> {
        &self.metadata
    }

    // ── Mutation ──────────────────────────────────────────────────────────────

    /// Drain and return all pending domain events.
    ///
    /// Uses `core::mem::take` — O(1), no allocation.  After this call
    /// `pending_events` is empty.
    pub fn drain_events(&mut self) -> Vec<CorpusEvent> {
        core::mem::take(&mut self.pending_events)
    }

    /// Insert a single vector into the corpus.
    ///
    /// # Errors
    ///
    /// - [`CorpusError::DimensionMismatch`] if `vector.len() != config.dimension()`.
    /// - [`CorpusError::DuplicateVectorId`] if `id` is already present.
    /// - [`CorpusError::Codec`] if the codec pipeline fails (only for `Compress` policy).
    pub fn insert(
        &mut self,
        id: VectorId,
        vector: &[f32],
        entry_metadata: Option<EntryMetaValue>,
        timestamp: Timestamp,
    ) -> Result<(), CorpusError> {
        self.validate_dimension(vector)?;
        if self.vectors.contains_key(id.as_ref()) {
            return Err(CorpusError::DuplicateVectorId { id });
        }

        let compressed = self.compress_vector(vector)?;
        let meta = Self::unwrap_meta(entry_metadata);
        let entry = VectorEntry::new(Arc::clone(&id), compressed, timestamp, meta);

        self.vectors.insert(Arc::clone(&id), entry);

        self.pending_events.push(CorpusEvent::VectorsInserted {
            corpus_id: Arc::clone(&self.corpus_id),
            vector_ids: Arc::from([id]),
            count: 1,
            timestamp,
        });

        Ok(())
    }

    /// Insert a batch of vectors atomically (all-or-nothing).
    ///
    /// On failure the corpus is unchanged and no events are emitted.
    /// On success, exactly one [`CorpusEvent::VectorsInserted`] is emitted
    /// covering all inserted ids.
    ///
    /// # Errors
    ///
    /// - [`CorpusError::BatchAtomicityFailure`] if any vector fails validation
    ///   or compression. The `index` field identifies the first failing vector.
    pub fn insert_batch(
        &mut self,
        vectors: &[(VectorId, &[f32], Option<EntryMetaValue>)],
        timestamp: Timestamp,
    ) -> Result<BatchReport, CorpusError> {
        // Stage 1: validate + compress all vectors without touching the map.
        let mut staged: Vec<(VectorId, VectorEntry)> = Vec::with_capacity(vectors.len());

        for (index, (id, vector, meta)) in vectors.iter().enumerate() {
            // Validate dimension.
            if let Err(e) = self.validate_dimension(vector) {
                return Err(CorpusError::BatchAtomicityFailure {
                    index,
                    source: Box::new(e),
                });
            }
            // Check for duplicates already in the corpus.
            if self.vectors.contains_key(id.as_ref()) {
                return Err(CorpusError::BatchAtomicityFailure {
                    index,
                    source: Box::new(CorpusError::DuplicateVectorId { id: Arc::clone(id) }),
                });
            }
            // Check for duplicates within this batch itself.
            let already_staged = staged
                .iter()
                .any(|(staged_id, _)| staged_id.as_ref() == id.as_ref());
            if already_staged {
                return Err(CorpusError::BatchAtomicityFailure {
                    index,
                    source: Box::new(CorpusError::DuplicateVectorId { id: Arc::clone(id) }),
                });
            }

            // Compress.
            let compressed = match self.compress_vector(vector) {
                Ok(cv) => cv,
                Err(e) => {
                    return Err(CorpusError::BatchAtomicityFailure {
                        index,
                        source: Box::new(CorpusError::Codec(e)),
                    });
                }
            };

            let entry_meta = Self::unwrap_meta(meta.clone());
            staged.push((
                Arc::clone(id),
                VectorEntry::new(Arc::clone(id), compressed, timestamp, entry_meta),
            ));
        }

        // Stage 2: commit — all validations passed, flip into the map.
        let mut ids: Vec<VectorId> = Vec::with_capacity(staged.len());
        let inserted_count = staged.len();
        for (id, entry) in staged {
            ids.push(Arc::clone(&id));
            self.vectors.insert(id, entry);
        }

        if !ids.is_empty() {
            #[allow(clippy::cast_possible_truncation)]
            let count = ids.len() as u32;
            self.pending_events.push(CorpusEvent::VectorsInserted {
                corpus_id: Arc::clone(&self.corpus_id),
                vector_ids: Arc::from(ids.into_boxed_slice()),
                count,
                timestamp,
            });
        }

        Ok(BatchReport {
            inserted: inserted_count,
            skipped: 0,
            first_error: None,
        })
    }

    /// Decompress a single vector by id.
    ///
    /// Does **not** emit a domain event (Python parity).
    ///
    /// # Errors
    ///
    /// - [`CorpusError::UnknownVectorId`] if `id` is not present.
    /// - [`CorpusError::Codec`] if the codec pipeline fails.
    pub fn decompress(&self, id: &VectorId) -> Result<Vec<f32>, CorpusError> {
        let entry = self
            .vectors
            .get(id.as_ref())
            .ok_or_else(|| CorpusError::UnknownVectorId { id: Arc::clone(id) })?;

        self.decompress_entry(entry)
    }

    /// Decompress all vectors and return them as an insertion-ordered map.
    ///
    /// Emits exactly one [`CorpusEvent::Decompressed`] on success.
    ///
    /// # Errors
    ///
    /// Propagates errors from the codec pipeline for any individual vector.
    pub fn decompress_all(&mut self) -> Result<BTreeMap<VectorId, Vec<f32>>, CorpusError> {
        let mut out = BTreeMap::new();
        for (id, entry) in self.vectors.iter() {
            let decompressed = self.decompress_entry(entry)?;
            out.insert(Arc::clone(id), decompressed);
        }

        #[allow(clippy::cast_possible_truncation)]
        let vector_count = out.len() as u32;

        self.pending_events.push(CorpusEvent::Decompressed {
            corpus_id: Arc::clone(&self.corpus_id),
            vector_count,
            timestamp: 0,
        });

        Ok(out)
    }

    /// Decompress all vectors and return them with an explicit timestamp.
    ///
    /// Emits exactly one [`CorpusEvent::Decompressed`] on success.
    ///
    /// # Errors
    ///
    /// Propagates errors from the codec pipeline for any individual vector.
    pub fn decompress_all_at(
        &mut self,
        timestamp: Timestamp,
    ) -> Result<BTreeMap<VectorId, Vec<f32>>, CorpusError> {
        let mut out = BTreeMap::new();
        for (id, entry) in self.vectors.iter() {
            let decompressed = self.decompress_entry(entry)?;
            out.insert(Arc::clone(id), decompressed);
        }

        #[allow(clippy::cast_possible_truncation)]
        let vector_count = out.len() as u32;

        self.pending_events.push(CorpusEvent::Decompressed {
            corpus_id: Arc::clone(&self.corpus_id),
            vector_count,
            timestamp,
        });

        Ok(out)
    }

    /// Remove a vector by id, returning the entry if present.
    ///
    /// Silent: no domain event is emitted (Python parity — `del corpus[id]`).
    pub fn remove(&mut self, id: &VectorId) -> Option<VectorEntry> {
        self.vectors.remove(id.as_ref())
    }

    // ── Private helpers ───────────────────────────────────────────────────────

    /// Validate that `vector.len()` matches the configured dimension.
    #[allow(clippy::missing_const_for_fn)] // calls non-const CodecConfig::dimension()
    fn validate_dimension(&self, vector: &[f32]) -> Result<(), CorpusError> {
        let expected = self.config.dimension();
        #[allow(clippy::cast_possible_truncation)]
        let got = vector.len() as u32;
        if got != expected {
            return Err(CorpusError::DimensionMismatch { expected, got });
        }
        Ok(())
    }

    /// Compress a vector slice according to the active policy.
    fn compress_vector(
        &self,
        vector: &[f32],
    ) -> Result<CompressedVector, crate::errors::CodecError> {
        match self.compression_policy {
            CompressionPolicy::Compress => {
                Codec::new().compress(vector, &self.config, &self.codebook)
            }
            CompressionPolicy::Passthrough => {
                // Store raw f32 bytes: 4 bytes per dimension in the `indices` field.
                // Each f32 is stored as 4 sequential index bytes (little-endian).
                let mut indices: Vec<u8> = Vec::with_capacity(vector.len() * 4);
                for &v in vector {
                    indices.extend_from_slice(&v.to_le_bytes());
                }
                let dim = vector.len();
                #[allow(clippy::cast_possible_truncation)]
                let dim_u32 = dim as u32;
                // We store 4 bytes per dimension in `indices` but the CompressedVector
                // invariant expects `indices.len() == dimension`. We set dimension to
                // byte_count (= dim * 4) to satisfy the invariant.
                let byte_len = indices.len();
                #[allow(clippy::cast_possible_truncation)]
                let byte_len_u32 = byte_len as u32;
                // Store byte_len_u32 as the CompressedVector dimension.
                // Decompression reads back 4 bytes per original f32 dimension.
                let _ = dim_u32; // original dim tracked via byte_len / 4 at decompress time
                CompressedVector::new(
                    indices.into_boxed_slice(),
                    None,
                    self.config.config_hash().clone(),
                    byte_len_u32,
                    8,
                )
            }
            CompressionPolicy::Fp16 => {
                // Store f16 LE: 2 bytes per dimension.
                let mut indices: Vec<u8> = Vec::with_capacity(vector.len() * 2);
                for &v in vector {
                    let h = f16::from_f32(v);
                    indices.extend_from_slice(&h.to_le_bytes());
                }
                let byte_len = indices.len();
                #[allow(clippy::cast_possible_truncation)]
                let byte_len_u32 = byte_len as u32;
                CompressedVector::new(
                    indices.into_boxed_slice(),
                    None,
                    self.config.config_hash().clone(),
                    byte_len_u32,
                    8,
                )
            }
        }
    }

    /// Decompress a [`VectorEntry`] according to the active policy.
    fn decompress_entry(&self, entry: &VectorEntry) -> Result<Vec<f32>, CorpusError> {
        match self.compression_policy {
            CompressionPolicy::Compress => Codec::new()
                .decompress(entry.compressed(), &self.config, &self.codebook)
                .map_err(CorpusError::Codec),
            CompressionPolicy::Passthrough => {
                // Reconstruct f32 from 4-byte chunks in `indices`.
                let bytes = entry.compressed().indices();
                if bytes.len() % 4 != 0 {
                    return Err(CorpusError::Codec(
                        crate::errors::CodecError::LengthMismatch {
                            left: bytes.len(),
                            right: (bytes.len() / 4) * 4,
                        },
                    ));
                }
                let floats: Vec<f32> = bytes
                    .chunks_exact(4)
                    .map(|chunk| {
                        // chunks_exact guarantees exactly 4 bytes; array indexing is safe.
                        #[allow(clippy::indexing_slicing)]
                        f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]])
                    })
                    .collect();
                Ok(floats)
            }
            CompressionPolicy::Fp16 => {
                // Reconstruct f32 from 2-byte f16 LE chunks.
                let bytes = entry.compressed().indices();
                if bytes.len() % 2 != 0 {
                    return Err(CorpusError::Codec(
                        crate::errors::CodecError::LengthMismatch {
                            left: bytes.len(),
                            right: (bytes.len() / 2) * 2,
                        },
                    ));
                }
                let floats: Vec<f32> = bytes
                    .chunks_exact(2)
                    .map(|chunk| {
                        #[allow(clippy::indexing_slicing)]
                        let h = f16::from_le_bytes([chunk[0], chunk[1]]);
                        f32::from(h)
                    })
                    .collect();
                Ok(floats)
            }
        }
    }

    /// Convert an `Option<EntryMetaValue>` to a metadata map.
    fn unwrap_meta(meta: Option<EntryMetaValue>) -> BTreeMap<String, EntryMetaValue> {
        match meta {
            None => BTreeMap::new(),
            Some(EntryMetaValue::Object(arc_map)) => {
                // Flatten an Object into the map — keys become string keys.
                arc_map
                    .iter()
                    .map(|(k, v)| (k.as_ref().to_string(), v.clone()))
                    .collect()
            }
            Some(other) => {
                // Non-object metadata stored under a sentinel key.
                let mut m = BTreeMap::new();
                m.insert("_value".to_string(), other);
                m
            }
        }
    }
}
