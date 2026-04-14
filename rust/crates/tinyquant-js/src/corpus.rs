//! napi-rs bindings for the `tinyquant-core` corpus aggregate.
//!
//! Surface mirrors `tinyquant_cpu.corpus` (Python) and
//! `rust/crates/tinyquant-py/src/corpus.rs` (PyO3) with camelCase
//! conventions noted in
//! `docs/plans/rust/phase-25-typescript-npm-package.md` §API parity
//! specification — corpus.
//!
//! Phase 25.3 scope: `CompressionPolicy`, `VectorEntry`, `Corpus`.
//! Batch paths are exposed via `insertBatch`; domain events are
//! serialised to JSON strings with a `type` discriminator so the TS
//! wrapper can `JSON.parse` them into typed objects without bridging
//! arbitrary JS values through napi-v2.
//!
//! All aggregate logic is delegated to `tinyquant_core::corpus::Corpus`.

use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};

use napi::bindgen_prelude::Float32Array;
use napi::{Error as NapiError, Status};
use napi_derive::napi;
use serde_json::json;

use tinyquant_core::corpus::{
    CompressionPolicy as CoreCompressionPolicy, Corpus as CoreCorpus,
    CorpusEvent as CoreCorpusEvent, VectorEntry as CoreVectorEntry,
};
use tinyquant_core::errors::CorpusError as CoreCorpusError;
use tinyquant_core::types::VectorId;

use crate::codec::{Codebook, CodecConfig, CompressedVector};
use crate::errors::map_corpus_error;

// ---------------------------------------------------------------------------
// CompressionPolicy
// ---------------------------------------------------------------------------

/// Python-parity compression policy.  napi-rs v2 cannot map Rust enums
/// directly to JS value constants, so the policy is modelled as a
/// `#[napi]` class with factory methods — the TS wrapper re-exposes
/// the three canonical instances as static readonly properties.
#[napi]
#[derive(Clone)]
pub struct CompressionPolicy {
    kind: CoreCompressionPolicy,
}

#[napi]
impl CompressionPolicy {
    /// Policy: full codec pipeline — rotate → scalar quantize →
    /// optional FP16 residual.
    #[napi(factory)]
    #[allow(clippy::self_named_constructors)]
    pub fn compress() -> Self {
        Self {
            kind: CoreCompressionPolicy::Compress,
        }
    }

    /// Policy: store raw FP32 bytes with no transformation.
    #[napi(factory)]
    pub fn passthrough() -> Self {
        Self {
            kind: CoreCompressionPolicy::Passthrough,
        }
    }

    /// Policy: cast each FP32 to IEEE-754 binary16 (FP16) and store
    /// 2 bytes per dimension.
    #[napi(factory)]
    pub fn fp16() -> Self {
        Self {
            kind: CoreCompressionPolicy::Fp16,
        }
    }

    /// Canonical lowercase tag: `"compress" | "passthrough" | "fp16"`.
    #[napi(getter)]
    pub fn kind(&self) -> &'static str {
        match self.kind {
            CoreCompressionPolicy::Compress => "compress",
            CoreCompressionPolicy::Passthrough => "passthrough",
            CoreCompressionPolicy::Fp16 => "fp16",
        }
    }

    /// True only for `Compress` — the other policies bypass the codec.
    #[napi]
    pub fn requires_codec(&self) -> bool {
        self.kind.requires_codec()
    }

    /// Storage dtype tag: `"uint8" | "float16" | "float32"`.
    #[napi]
    pub fn storage_dtype(&self) -> &'static str {
        match self.kind {
            CoreCompressionPolicy::Compress => "uint8",
            CoreCompressionPolicy::Fp16 => "float16",
            CoreCompressionPolicy::Passthrough => "float32",
        }
    }
}

impl CompressionPolicy {
    pub(crate) fn core(&self) -> CoreCompressionPolicy {
        self.kind
    }
}

// ---------------------------------------------------------------------------
// VectorEntry
// ---------------------------------------------------------------------------

/// Entity representing one compressed vector stored in the corpus.
#[napi]
pub struct VectorEntry {
    pub(crate) inner: CoreVectorEntry,
}

#[napi]
impl VectorEntry {
    #[napi(getter)]
    pub fn vector_id(&self) -> String {
        self.inner.vector_id().to_string()
    }

    /// Clone of the underlying compressed payload.
    #[napi]
    pub fn compressed(&self) -> CompressedVector {
        CompressedVector {
            inner: self.inner.compressed().clone(),
        }
    }

    #[napi(getter)]
    pub fn config_hash(&self) -> String {
        self.inner.config_hash().to_string()
    }

    /// True f32 vector dimension (not the byte count that
    /// `CompressedVector.dimension` reports for the Passthrough /
    /// Fp16 policies).
    #[napi(getter)]
    pub fn dimension(&self) -> u32 {
        self.inner.dimension()
    }

    #[napi(getter)]
    pub fn has_residual(&self) -> bool {
        self.inner.has_residual()
    }

    /// Insertion timestamp as milliseconds since the Unix epoch.
    ///
    /// The core stores a nanosecond `i64` for `no_std` compatibility;
    /// we narrow to `f64` milliseconds at the FFI boundary so the TS
    /// wrapper can build a `Date` without bridging chrono / BigInt.
    #[napi(getter)]
    pub fn inserted_at_millis(&self) -> f64 {
        timestamp_nanos_to_millis(self.inner.inserted_at())
    }

    /// Entry metadata serialised as a JSON string, or `null` when empty.
    ///
    /// Phase 25.3 deviation: metadata is not yet threaded through
    /// `CoreCorpus` end-to-end. `EntryMetaValue` is not `serde`-serializable
    /// from this crate (it is `no_std + alloc` and avoids a `serde_json`
    /// dependency) and the PyO3 binding takes the same stance — it raises
    /// `NotImplementedError` on metadata reads/writes. Rather than render
    /// a lossy `format!("{v:?}")` shape that silently corrupts
    /// `Integer(5)` → `"Integer(5)"`, this getter unconditionally returns
    /// `None`. Inputs to `insert` / `insertBatch` are documented no-ops;
    /// Phase 25.4 or beyond wires metadata through properly once the core
    /// lands a JSON round-trip for `EntryMetaValue`.
    #[napi(getter)]
    pub fn metadata_json(&self) -> Option<String> {
        None
    }
}

// ---------------------------------------------------------------------------
// Corpus
// ---------------------------------------------------------------------------

/// Aggregate root managing a collection of compressed vectors.
///
/// Wraps `tinyquant_core::corpus::Corpus` behind an `Arc<Mutex<_>>`
/// so `&self` JS methods can still mutate the underlying aggregate —
/// JS callers always have a single thread of control but napi-rs
/// constrains `#[napi] impl` methods to `&self`.
#[napi]
pub struct Corpus {
    inner: Arc<Mutex<CoreCorpus>>,
    codec_config: CodecConfig,
    codebook: Codebook,
    compression_policy: CompressionPolicy,
    corpus_id: String,
    metadata_json: Option<String>,
}

#[napi]
impl Corpus {
    /// Construct a new `Corpus`. Parameters are taken individually
    /// rather than as an `#[napi(object)]` struct because the latter
    /// cannot carry `#[napi]` class references as fields in v2; the
    /// TS wrapper re-exposes the options-object shape documented in
    /// `§src/corpus.ts`.
    #[napi(constructor)]
    pub fn new(
        corpus_id: String,
        codec_config: &CodecConfig,
        codebook: &Codebook,
        compression_policy: &CompressionPolicy,
        metadata_json: Option<String>,
    ) -> napi::Result<Self> {
        let core = CoreCorpus::new(
            VectorId::from(corpus_id.as_str()),
            codec_config.inner.clone(),
            codebook.inner.clone(),
            compression_policy.core(),
            BTreeMap::new(),
        );
        Ok(Self {
            inner: Arc::new(Mutex::new(core)),
            codec_config: CodecConfig {
                inner: codec_config.inner.clone(),
            },
            codebook: Codebook {
                inner: codebook.inner.clone(),
            },
            compression_policy: compression_policy.clone(),
            corpus_id,
            metadata_json,
        })
    }

    #[napi(getter)]
    pub fn corpus_id(&self) -> String {
        self.corpus_id.clone()
    }

    /// Clone of the codec config the corpus was constructed with.
    #[napi]
    pub fn codec_config(&self) -> CodecConfig {
        CodecConfig {
            inner: self.codec_config.inner.clone(),
        }
    }

    /// Clone of the codebook the corpus was constructed with.
    #[napi]
    pub fn codebook(&self) -> Codebook {
        Codebook {
            inner: self.codebook.inner.clone(),
        }
    }

    /// Clone of the active compression policy.
    #[napi]
    pub fn compression_policy(&self) -> CompressionPolicy {
        self.compression_policy.clone()
    }

    /// Construction metadata as a JSON string (or `null`).
    #[napi(getter)]
    pub fn metadata_json(&self) -> Option<String> {
        self.metadata_json.clone()
    }

    #[napi(getter)]
    pub fn vector_count(&self) -> napi::Result<u32> {
        let guard = self.inner.lock().map_err(lock_err)?;
        #[allow(clippy::cast_possible_truncation)]
        Ok(guard.vector_count() as u32)
    }

    #[napi(getter)]
    pub fn is_empty(&self) -> napi::Result<bool> {
        let guard = self.inner.lock().map_err(lock_err)?;
        Ok(guard.is_empty())
    }

    /// List of vector ids in insertion order.  The TS wrapper wraps
    /// this in a `ReadonlySet<string>`.
    #[napi(getter)]
    pub fn vector_ids(&self) -> napi::Result<Vec<String>> {
        let guard = self.inner.lock().map_err(lock_err)?;
        Ok(guard.iter().map(|(id, _)| id.to_string()).collect())
    }

    /// Insert a single vector.  The `_metadata_json` parameter is
    /// accepted for API-shape parity but ignored until the core lands
    /// a `serde` round-trip for `EntryMetaValue` (see `metadata_json`
    /// getter docstring for the Phase 25.3 deviation rationale).
    #[napi]
    pub fn insert(
        &self,
        vector_id: String,
        vector: Float32Array,
        _metadata_json: Option<String>,
    ) -> napi::Result<VectorEntry> {
        let slice: &[f32] = vector.as_ref();
        // napi Float32Array borrow dies at call return; own the buffer.
        let buf = slice.to_vec();
        let id = VectorId::from(vector_id.as_str());
        let id_for_lookup = id.clone();
        // Hold the lock across both the mutation and the read-back so
        // no other call can observe / mutate the corpus between the two
        // steps. `&self` methods serialise through this single mutex;
        // napi-rs v2 will only marshal one JS call at a time, but being
        // explicit here costs nothing and documents the invariant.
        let mut guard = self.inner.lock().map_err(lock_err)?;
        guard
            .insert(id, &buf, None, timestamp_now_nanos())
            .map_err(map_corpus_error)?;
        let entry = guard
            .iter()
            .find(|(eid, _)| eid.as_ref() == id_for_lookup.as_ref())
            .map(|(_, e)| e.clone())
            .ok_or_else(|| {
                NapiError::new(
                    Status::GenericFailure,
                    "insert succeeded but entry missing".to_string(),
                )
            })?;
        Ok(VectorEntry { inner: entry })
    }

    /// Batch insert.  `vectors` is a JS object map of id → Float32Array;
    /// metadata is intentionally not supported yet (the PyO3 binding
    /// has the same limitation, per Phase 22.A code review).  On error
    /// the corpus is unchanged.
    #[napi]
    pub fn insert_batch(
        &self,
        vectors: HashMap<String, Float32Array>,
        _metadata_json: Option<String>,
    ) -> napi::Result<Vec<VectorEntry>> {
        // Stage owned copies first so we can form `(VectorId, &[f32])`
        // tuples that live for the duration of the core call.
        // napi Float32Array borrow dies at call return; own the buffer.
        let mut owned: Vec<(VectorId, Vec<f32>)> = Vec::with_capacity(vectors.len());
        for (k, v) in vectors {
            let s: &[f32] = v.as_ref();
            owned.push((VectorId::from(k.as_str()), s.to_vec()));
        }
        let ids_for_lookup: Vec<VectorId> = owned.iter().map(|(id, _)| id.clone()).collect();

        let batch: Vec<(VectorId, &[f32], Option<tinyquant_core::corpus::EntryMetaValue>)> = owned
            .iter()
            .map(|(id, v)| (id.clone(), v.as_slice(), None))
            .collect();

        // Hold the lock across the insert AND the read-back so callers
        // never observe a half-committed corpus. `CoreCorpus::insert_batch`
        // returns a `BatchReport` (counts only, not the inserted entries)
        // so we still need to walk the map once per id to clone each
        // `VectorEntry`.  That walk stays O(n·m) in the worst case but
        // it runs under a single lock and matches the PyO3 binding's
        // shape; the typical batch size for JS consumers (few hundred
        // vectors) keeps this well inside acceptable latency.
        let mut guard = self.inner.lock().map_err(lock_err)?;
        guard
            .insert_batch(&batch, timestamp_now_nanos())
            .map_err(map_corpus_error)?;

        // Collect entries in the order the caller passed them.
        let mut out: Vec<VectorEntry> = Vec::with_capacity(ids_for_lookup.len());
        for id in ids_for_lookup {
            let entry = guard
                .iter()
                .find(|(eid, _)| eid.as_ref() == id.as_ref())
                .map(|(_, e)| e.clone())
                .ok_or_else(|| {
                    NapiError::new(
                        Status::GenericFailure,
                        format!("insertBatch: staged id {id:?} missing after commit"),
                    )
                })?;
            out.push(VectorEntry { inner: entry });
        }
        Ok(out)
    }

    /// Look up a vector entry by id.
    #[napi]
    pub fn get(&self, vector_id: String) -> napi::Result<VectorEntry> {
        let guard = self.inner.lock().map_err(lock_err)?;
        let id = VectorId::from(vector_id.as_str());
        let entry = guard
            .iter()
            .find(|(eid, _)| eid.as_ref() == id.as_ref())
            .map(|(_, e)| e.clone());
        match entry {
            Some(e) => Ok(VectorEntry { inner: e }),
            // Route through `map_corpus_error` so the
            // `"<ClassName>: reason"` contract stays centralised.
            None => Err(map_corpus_error(CoreCorpusError::UnknownVectorId {
                id: Arc::clone(&id),
            })),
        }
    }

    #[napi]
    pub fn contains(&self, vector_id: String) -> napi::Result<bool> {
        let guard = self.inner.lock().map_err(lock_err)?;
        Ok(guard.contains(&VectorId::from(vector_id.as_str())))
    }

    #[napi]
    pub fn decompress(&self, vector_id: String) -> napi::Result<Float32Array> {
        let guard = self.inner.lock().map_err(lock_err)?;
        let id = VectorId::from(vector_id.as_str());
        let out = guard.decompress(&id).map_err(map_corpus_error)?;
        Ok(Float32Array::new(out))
    }

    /// Decompress every vector.  Returns a JS object map `id →
    /// Float32Array` — matches the Python `decompress_all()` shape.
    #[napi]
    pub fn decompress_all(&self) -> napi::Result<HashMap<String, Float32Array>> {
        let mut guard = self.inner.lock().map_err(lock_err)?;
        let map = guard
            .decompress_all_at(timestamp_now_nanos())
            .map_err(map_corpus_error)?;
        let mut out: HashMap<String, Float32Array> = HashMap::with_capacity(map.len());
        for (id, vec) in map {
            out.insert(id.to_string(), Float32Array::new(vec));
        }
        Ok(out)
    }

    /// Remove a vector silently — matches Python `del corpus[id]`.
    /// Returns `true` when the id was present, `false` otherwise.
    #[napi]
    pub fn remove(&self, vector_id: String) -> napi::Result<bool> {
        let mut guard = self.inner.lock().map_err(lock_err)?;
        let id = VectorId::from(vector_id.as_str());
        Ok(guard.remove(&id).is_some())
    }

    /// Drain all pending domain events and return each as a JSON
    /// string with a `type` discriminator.  The TS wrapper
    /// `JSON.parse`s these into typed `CorpusEvent[]`.
    #[napi]
    pub fn pending_events(&self) -> napi::Result<Vec<String>> {
        let mut guard = self.inner.lock().map_err(lock_err)?;
        let events = guard.drain_events();
        Ok(events.into_iter().map(event_to_json_string).collect())
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn lock_err<T>(_err: std::sync::PoisonError<T>) -> NapiError {
    NapiError::new(
        Status::GenericFailure,
        "TinyQuantError: corpus mutex poisoned".to_string(),
    )
}

/// Nanosecond → millisecond conversion as `f64`.  Nanosecond precision
/// is lost, but that matches `Date`'s millisecond resolution.
fn timestamp_nanos_to_millis(nanos: i64) -> f64 {
    // f64 has 53-bit mantissa — millisecond timestamps stay exact
    // through the year ≈ 285 727 AD, well beyond any realistic
    // insertion timestamp.
    nanos as f64 / 1_000_000.0
}

/// Current wall-clock time in nanoseconds since the Unix epoch.
///
/// Phase 25.3 corpus calls don't expose a timestamp parameter on the
/// JS surface (matching the Python fat wheel), so we sample
/// `SystemTime::now()` on the Rust side.  The TS wrapper stays
/// deterministic-friendly by exposing millisecond floats via
/// `insertedAtMillis`, which consumers can compare to `Date.now()`.
fn timestamp_now_nanos() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(d) => i64::try_from(d.as_nanos()).unwrap_or(i64::MAX),
        Err(_) => 0,
    }
}

fn event_to_json_string(event: CoreCorpusEvent) -> String {
    let value = match event {
        CoreCorpusEvent::Created {
            corpus_id,
            codec_config: _cfg,
            compression_policy,
            timestamp,
        } => {
            let policy = match compression_policy {
                CoreCompressionPolicy::Compress => "compress",
                CoreCompressionPolicy::Passthrough => "passthrough",
                CoreCompressionPolicy::Fp16 => "fp16",
            };
            json!({
                "type": "CorpusCreated",
                "corpusId": corpus_id.to_string(),
                "compressionPolicy": policy,
                "timestampMillis": timestamp_nanos_to_millis(timestamp),
            })
        }
        CoreCorpusEvent::VectorsInserted {
            corpus_id,
            vector_ids,
            count,
            timestamp,
        } => {
            let ids: Vec<String> = vector_ids.iter().map(|v| v.to_string()).collect();
            json!({
                "type": "VectorsInserted",
                "corpusId": corpus_id.to_string(),
                "vectorIds": ids,
                "count": count,
                "timestampMillis": timestamp_nanos_to_millis(timestamp),
            })
        }
        CoreCorpusEvent::Decompressed {
            corpus_id,
            vector_count,
            timestamp,
        } => json!({
            "type": "CorpusDecompressed",
            "corpusId": corpus_id.to_string(),
            "vectorCount": vector_count,
            "timestampMillis": timestamp_nanos_to_millis(timestamp),
        }),
        CoreCorpusEvent::PolicyViolationDetected {
            corpus_id,
            kind,
            detail,
            timestamp,
        } => json!({
            "type": "CompressionPolicyViolationDetected",
            "corpusId": corpus_id.to_string(),
            "violationType": kind.as_python_tag(),
            "detail": detail.to_string(),
            "timestampMillis": timestamp_nanos_to_millis(timestamp),
        }),
    };
    value.to_string()
}
