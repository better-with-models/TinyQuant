# TinyQuant Integration with better-router

**Date:** 2026-04-08
**Status:** Research seed
**Depends on:** [TinyQuant README](README.md),
[TurboSwede Integration](../turboswede/better-router-integration.md)
**Upstream refs:** E37-E38 (TurboQuant validation), E39 (router-wide
deployment validation), E24 (memory management), E21 (gold corpus),
E49b (quality estimation hierarchy)

## Integration Context

The better-router inference engine stores high-dimensional embeddings
across multiple subsystems:

| Subsystem | Dimension | Embedding model | Volume (30 days) |
|-----------|-----------|----------------|-----------------|
| Decision Context Store | 384 | all-MiniLM-L6-v2 | ~3.8 GB |
| Gold Corpus | 3072 | text-embedding-3-large | ~1.2 GB |
| Orchestrator Journal | 3072 | text-embedding-3-large | ~4.5 GB |
| Routing Performance | 3072 | text-embedding-3-large | ~2.1 GB |
| Calibration History | 3072 | text-embedding-3-large | ~1.8 GB |
| Traffic Patterns | 384 | all-MiniLM-L6-v2 | ~2.5 GB |
| **Total** | | | **~15.9 GB** |

Without compression, the vector storage grows to ~15.9 GB over 30 days
(E38 Part E projection). With TinyQuant 4-bit compression, this drops to
~2.0 GB (87% reduction), and the effective memory depth extends from
~7 days to 30+ days before compaction is needed.

TinyQuant is consumed by better-router exclusively through TurboSwede
(the OpenViking-compatible context database). The router never calls
TinyQuant directly — TurboSwede handles compression and decompression
as part of its storage pipeline.

---

## Deployment Targets

Ordered by confidence level from E37-E38 experimental validation:

### 1. Gold Corpus (HIGH CONFIDENCE)

**What:** The gold corpus stores multi-provider reference responses with
embeddings used for quality calibration. The gold similarity flywheel
(E21) computes embedding similarity between new responses and gold
references to estimate response quality without an LLM judge.

**Why TinyQuant works here:** Gold corpus quality depends on **score
correlation** (Pearson rho between FP32 and compressed similarity
scores), not on exact top-1 retrieval. E38 Part C measured Pearson
rho=0.997 for gold score fidelity — the quality estimation pipeline
is unaffected by compression.

**Quantitative evidence:**

- Pearson score correlation: 0.997 (target was 0.98) — PASS
- Scale-invariant: correlation holds at 500 and 3000 corpus sizes
- Gold similarity rho=0.82 feeds into the calibration flywheel (E09,
  E23) — the 0.3% fidelity loss from TinyQuant is well below the
  flywheel's own noise floor

**Integration pattern:**

```text
Gold reference arrives → embed(3072-d FP32) →
  TinyQuant.compress(4-bit) → TurboSwede stores bytea →
  ...later...
  Calibration query → TurboSwede decompresses →
  pgvector cosine similarity → score (rho=0.997 vs FP32)
```

**Deploy first.** This collection is read-heavy, write-rare, and
completely insensitive to top-1 noise.

### 2. Orchestrator Collections (MEDIUM CONFIDENCE)

**What:** The orchestrator agent (E22-E27) maintains several collections
in the context database:

- `orchestrator-journal`: Per-tick observations, insights, and
  reflections. Write-heavy append log, occasionally queried for
  cross-cycle continuity.
- `calibration-history`: Curve parameters and effectiveness records.
  Time-series pattern, queried by recency.
- `routing-performance`: Per-(model, category) quality profiles.
  Queried by structured filters (model name, category), not by
  vector similarity.
- `traffic-patterns`: Hourly/daily traffic distributions. Structured
  data with optional embedding column.

**Why TinyQuant works here:** These collections are primarily queried
by structured filters (model, category, timestamp), not by embedding
similarity. The vectors are secondary retrieval signals. Compression
saves storage without affecting the primary query path.

**Quantitative evidence:**

- Rank correlation 0.98+ at all tested scales — when vector search
  is used, approximate neighborhood retrieval is sufficient
- Write latency (7ms/vec at d=3072) is acceptable on the async
  orchestrator heartbeat path (5-minute cadence at the fastest tier)
- Storage savings extend memory depth from ~7 days to 30+ days,
  which directly benefits the orchestrator's cross-cycle analysis

**Integration pattern:**

```text
Orchestrator heartbeat → compute embedding →
  TinyQuant.compress(4-bit) → TurboSwede stores bytea →
  ...later...
  Self-retrieval query → TurboSwede decompresses →
  pgvector HNSW (or SQL filter) → journal entries
```

### 3. Decision Memory (CONDITIONAL)

**What:** The Decision Context Store records every routing decision's
outcome: which backend was chosen, what features were extracted, what
quality score was observed. Speculation (E15-E17) queries this store
to predict future routing decisions based on similar past requests.

**Why this is conditional:** Speculation accuracy depends on finding
the *exact* best historical match. Top-1 agreement at 5000 vectors is
only 15% (E38 Part C Full). However:

- Speculation accuracy without TinyQuant is already ~70% (E15), and
  the Thompson ceiling means even perfect speculation doesn't improve
  routing quality beyond warm-start convergence (E27B)
- Top-5 overlap is 80%+, and the speculation engine considers multiple
  candidates, not just top-1
- E54 confirmed that fingerprint-aware speculation (S3) actually
  *underperforms* the default by 8.1pp — the speculation engine is
  robust to retrieval noise

**Recommendation:** The E37-E38 findings recommend keeping decision
memories at FP16 (uncompressed). However, E54 provides new evidence:
fingerprint-aware speculation (S3) underperforms the default by 8.1pp,
and the Thompson ceiling means even perfect speculation doesn't improve
routing quality. This suggests the speculation engine is robust to
retrieval noise, making compression viable.

**Revised position:** Start with FP16 (PASSTHROUGH policy) per the
original E37-E38 recommendation. If storage pressure grows and
speculation quality remains stable, switch to COMPRESS. The
per-collection compression policy makes this a configuration change,
not a code change. Decision memories represent ~20% of total volume.

### 4. Context Recall (CONDITIONAL)

**What:** The context memory recall path (stage 6 of the request
lifecycle) queries stored interactions to inject relevant history
into the current request. This is the hot path — it runs on every
inference request.

**Why this is conditional:** Context recall injects the top-5 recalled
memories, not just top-1. The 80% top-5 overlap at d=3072 (E38 Part B)
means most relevant memories still appear in the result set. But:

- The token budget drives which tier (L0/L1/L2) each memory gets —
  slight reordering of results changes which memories get deeper
  context injection
- The recall query is time-bounded (must fit within the <10ms routing
  overhead budget), and decompress latency adds cost

**Mitigation strategy:** TurboSwede's decompress-then-HNSW-rerank
pipeline handles this. On write, compress to `bytea`. On read:

1. Decompress the HNSW index candidates (pgvector operates on a
   materialized decompressed shadow column or decompresses on demand)
2. pgvector HNSW search returns top-20 candidates at FP32 precision
3. Rerank top-20 to top-5 using exact cosine similarity
4. Return top-5 with L0/L1/L2 tiers

The decompression cost is amortized into TurboSwede's background
maintenance (the shadow column is refreshed periodically), not charged
to the hot-path query.

---

## Integration Points

### TinyQuant in the Request Lifecycle

TinyQuant participates in two stages of the 10-stage request lifecycle,
both mediated through TurboSwede:

**Stage 6 — Context Recall (read path):**

```text
Router calls TurboSwede: POST /api/v1/search/find
  → TurboSwede queries pgvector HNSW on decompressed shadow column
  → Returns L0/L1/L2 tiered results within token budget
  → TinyQuant is NOT on the hot path (decompression is pre-materialized)
```

**Stage 9 — Context Capture (write path, async):**

```text
Router calls TurboSwede: POST /api/v1/sessions/{id}/messages
  → TurboSwede receives FP32 embedding
  → TurboSwede calls TinyQuant.compress(embedding) → bytea
  → INSERT into PostgreSQL (compressed bytea + content)
  → Background job: decompress → update pgvector shadow column
  → TinyQuant latency (7ms at d=3072) is acceptable on async path
```

### TinyQuant in the Orchestrator

The orchestrator agent runs on heartbeat cadences (1 min to 6 hr).
TinyQuant participates in:

- **CompactMemories (30 min cadence):** Merge, deduplicate, and
  re-tier stored memories. Compaction operates on decompressed vectors,
  then re-compresses the result. TinyQuant's compress is called once
  per surviving memory after compaction.
- **ReindexDecisionMemories (6 hr cadence):** When the embedding model
  changes, all vectors must be re-embedded and re-compressed. TinyQuant's
  codebook must be recomputed for the new dimension if it changes.
- **PruneStaleGoldReferences (6 hr cadence):** Marks old gold references
  as stale. No TinyQuant involvement (metadata operation only).

### The SearchBackend Contract

TurboSwede implements TinyQuant's `SearchBackend` protocol via a
`PgVectorBackend` class:

```python
class PgVectorBackend(SearchBackend):
    """Search backend that delegates to pgvector HNSW.

    Vectors are stored as TinyQuant-compressed bytea in PostgreSQL.
    A materialized shadow column holds the decompressed FP32 vector
    for pgvector HNSW indexing. Search operates on the shadow column.
    """

    def search(self, query, corpus, k):
        # Query pgvector directly — shadow column is already FP32
        # TinyQuant compression is transparent to the search path
        results = self.db.execute(
            "SELECT id, metadata, 1 - (embedding <=> %s) AS score "
            "FROM embeddings "
            "ORDER BY embedding <=> %s LIMIT %s",
            [query, query, k]
        )
        return [SearchResult(r.id, r.score, r.metadata) for r in results]
```

---

## Configuration Contract

The better-router's `RouterConfig` gains TinyQuant-related fields,
mediated through TurboSwede's configuration:

```typescript
interface RouterConfig {
  // ... existing fields ...

  /** TurboSwede connection (replaces contextMemoryUrl) */
  contextMemoryUrl: string;  // default: "http://turboswede:1933"

  // TinyQuant settings are TurboSwede-internal, not router-level.
  // The router doesn't know about compression — TurboSwede handles it.
}
```

TurboSwede's own configuration exposes TinyQuant settings:

```python
@dataclass
class TurboSwedeConfig:
    # TinyQuant compression settings
    tinyquant_bits: int = 4           # bit depth (2, 3, or 4)
    tinyquant_seed: int = 42          # deterministic rotation matrix
    tinyquant_enabled: bool = True    # global enable/disable

    # Per-collection compression policy
    collection_policies: dict[str, CompressionPolicy] = field(
        default_factory=lambda: {
            "gold-corpus": CompressionPolicy.COMPRESS,
            "orchestrator-journal": CompressionPolicy.COMPRESS,
            "routing-performance": CompressionPolicy.COMPRESS,
            "calibration-history": CompressionPolicy.COMPRESS,
            "decision-memory": CompressionPolicy.PASSTHROUGH,
            "context-recall": CompressionPolicy.COMPRESS,
        }
    )

class CompressionPolicy(Enum):
    COMPRESS = "compress"       # TinyQuant 4-bit
    PASSTHROUGH = "passthrough" # Store at native precision (FP32)
    FP16 = "fp16"               # Store at half precision (no TinyQuant)
```

The per-collection policy allows exempting specific collections from
compression (e.g., decision memory at FP16) without code changes.

---

## Performance Budget

### Write Path (Stage 9, async — not latency-critical)

| Operation | Latency | Notes |
|-----------|---------|-------|
| TinyQuant compress (d=3072) | ~7 ms | Dominated by rotation matrix multiply (measured E38) |
| TinyQuant compress (d=384) | ~0.08 ms | Fast at lower dimensions (measured E37) |
| PostgreSQL INSERT (bytea) | ~1-2 ms | Single row insert |
| **Total write** | **~8-9 ms** | Async, doesn't block response |

The write path runs asynchronously after the response is sent to the
client (stage 9). The 7ms compress latency at d=3072 is well within
the async budget.

### Read Path (Stage 6, hot path — latency-critical)

| Operation | Latency | Notes |
|-----------|---------|-------|
| pgvector HNSW search | ~1.5-5 ms | On decompressed shadow column |
| L0/L1/L2 content fetch | ~1-2 ms | SQL JOIN with content table |
| **Total read** | **~3-7 ms** | Within <10ms routing overhead budget |

The read path does NOT decompress on demand. The decompressed shadow
column is maintained by a background job, so pgvector searches FP32
vectors directly. TinyQuant latency is zero on the hot path.

### Background Decompression (shadow column maintenance)

| Operation | Latency | Cadence |
|-----------|---------|---------|
| Decompress 1 vector (d=3072) | ~7 ms | Per new insert |
| Batch decompress 100 vectors | ~700 ms | Periodic catch-up |
| Full reindex (10,000 vectors) | ~70 s | After embedding model change |

Shadow column updates run as PostgreSQL triggers or background jobs.
New inserts trigger a single-vector decompress (~7ms). Batch catch-up
handles any backlog during high-write periods.

---

## Risks and Mitigations

### Codebook Drift on Embedding Model Change

**Risk:** TinyQuant's Lloyd-Max codebook is computed for the coordinate
distribution of vectors at a specific dimension. If the embedding model
changes (e.g., from text-embedding-3-large to a future model with
different dimension or distribution), the existing codebook may produce
suboptimal quantization.

**Mitigation:** The codebook is dimension-dependent, not
model-dependent. The coordinate distribution after random rotation
depends only on `d` (Lemma 1 in the paper). Changing embedding models
at the same dimension requires no codebook update. Changing dimension
requires recomputing the codebook (a one-time ~100ms operation).

### Seed Management

**Risk:** The rotation matrix Pi and QJL projection matrix S are
generated deterministically from a seed. Using different seeds for
compress and decompress produces garbage.

**Mitigation:** The seed is stored in the CompressedVector header and
in TurboSwede's configuration. All vectors in a collection share the
same seed. Seed is validated on decompress — mismatch raises an error
rather than returning a corrupted vector.

### Top-1 Degradation Scaling Law

**Risk:** As corpus size grows, top-1 agreement degrades as
O(1/sqrt(n)). At 10,000 vectors, top-1 at d=3072 would be ~10-12%
(extrapolated from the measured scaling law).

**Mitigation:** TinyQuant is positioned as a storage codec, not a
precision retrieval tool. The search path operates on decompressed FP32
vectors via pgvector HNSW, so top-1 is exact on the search side. The
scaling law only matters if someone searches on compressed vectors
directly, which TinyQuant's architecture explicitly prevents.

### Decompression Latency Under Load

**Risk:** During high-write periods (e.g., orchestrator batch
operations), the shadow column maintenance job could fall behind,
causing pgvector to search stale decompressed vectors.

**Mitigation:** The shadow column is eventually consistent by design.
New vectors are searchable after the next background decompress cycle
(configurable cadence, default 1 second). For the context memory use
case, a 1-second lag between capture and searchability is acceptable —
the same interaction is never recalled in the same request.
