# TurboQuant Integration: Extreme Compression for the Inference Router Stack

**Date:** April 6, 2026
**Status:** Research — experiments E37-E39 designed to validate
**Source:** [Google Research Blog](https://research.google/blog/turboquant-redefining-ai-efficiency-with-extreme-compression/), [turbo-quant-deep-research-report.md](turbo-quant-deep-research-report.md)
**Related:** [Context Memory](../design/inference-router/context-memory.md), [Adaptive Routing Engine](../design/inference-router/adaptive-routing-engine.md), [Gold Corpus (E21)](../plan/experiment-E21-provider-response-corpus.md), [OpenViking Learning Substrate (E20)](../plan/experiment-E20-openviking-speculative-memory.md)

---

## 1. What TurboQuant Is

TurboQuant is a two-stage compression algorithm from Google Research
that quantizes high-dimensional vectors to 2-4 bits with near-zero
distortion:

**Stage 1 — PolarQuant:** Random rotation of the data vector followed
by optimal scalar quantization of each coordinate. The rotation makes
the coordinate distribution predictable and concentrated, eliminating
per-block normalization overhead (~1 bit saved).

**Stage 2 — QJL (Quantized Johnson-Lindenstrauss):** A 1-bit random
projection of the quantization residual that preserves inner products
in expectation. Data-oblivious, GPU-friendly, zero memory overhead
beyond 1 bit per dimension.

**Key results:**

- KV cache: ≥6× compression at 3 bits, zero accuracy loss on
  long-context benchmarks (LongBench, NeedleInAHaystack)
- Attention speedup: up to 8× on H100 with 4-bit keys
- Vector search: higher recall@k than product quantization (PQ) and
  RabbiQ at equal bit budgets on GloVe-200d
- Theoretical: distortion within a constant factor of Shannon's
  information-theoretic lower bound
- Does NOT require retraining or fine-tuning
- Triton kernels available; vLLM integration referenced

**What TurboQuant does NOT do:**

- Weight quantization (use GPTQ/AWQ for that)
- Training-time optimization (inference-only)
- Small-model optimization (benefits scale with dimension)

---

## 2. Integration Points with better-router

TurboQuant intersects the better-router architecture in five distinct
areas, ranging from direct application (KV cache on local nodes) to
novel uses (vector compression in OpenViking):

### 2.1 Local LM Studio Node KV Cache Compression

**Where:** The 3 LM Studio nodes (10.60.1.x) running local models
(deepseek-8b, qwen-35b, gpt-oss-20b, etc.)

**Current state:** LM Studio serves models with default KV cache
precision (FP16). Long context windows consume substantial VRAM,
limiting concurrent model loading and maximum context length.

**TurboQuant opportunity:** If TurboQuant kernels can be integrated
into the inference backend (llama.cpp or the LM Studio serving
layer), each node could:

- Hold ~6× longer contexts at the same VRAM budget
- Load more models simultaneously (KV cache is a major VRAM consumer)
- Serve faster attention (8× speedup on compute-bound long contexts)

**Router impact:** More available models per node → richer routing
pool → better bandit decisions. Longer contexts → fewer provider
transitions due to context window exhaustion. Faster attention →
lower latency → better composite reward scores.

**Challenge:** LM Studio may not expose the hooks needed to swap KV
cache quantization. This may require a custom inference backend
(vLLM with TurboQuant kernels) or waiting for LM Studio support.

### 2.2 OpenViking Vector Index Compression

**Where:** OpenViking stores embeddings for semantic search — every
conversation memory, decision outcome, journal entry, and gold
reference is embedded and indexed.

**Current state:** OpenViking uses text-embedding-3-large (dimension
3072) at full precision. As memories accumulate (E20, E24), the
vector index grows. At 3072 dimensions × FP32 (4 bytes) = 12.3KB
per vector. With 10,000 memories, that's 123MB of vector data alone,
plus index overhead.

**TurboQuant opportunity:** Compress the vector index from FP32 to
3-4 bits per dimension:

- 3072 dims × 3 bits = 1.15KB per vector (10.7× compression)
- 10,000 memories: 11.5MB instead of 123MB
- Vector search recall preserved (TurboQuant outperforms PQ on
  GloVe at equal bit budgets)
- Semantic retrieval latency reduced (less data to scan)

**Router impact:** OpenViking can store 10× more memories in the same
footprint. Context recall (stage 6) is faster. The Orchestrator's
journal (E22-E27) can be more verbose without storage concern. Gold
corpus (E21) can grow larger before staleness pruning is needed.

**Challenge:** OpenViking's internal vector index (likely FAISS or
similar) would need to support TurboQuant's two-stage representation.
This might require a custom FAISS index type or a wrapper that
quantizes on insert and dequantizes on search.

### 2.3 Feature Extraction Pipeline: Compressed Semantic Embeddings

**Where:** The adaptive routing engine's feature extraction pipeline
(adaptive-routing-engine.md) has a deferred Tier 4: semantic
embedding using all-MiniLM-L6-v2 (384 dimensions). This was deferred
because of latency concerns (predicted 5-15ms).

**TurboQuant opportunity:** TurboQuant-compressed semantic embeddings
could make Tier 4 viable:

- Compute embedding at full precision (one-time cost)
- Compress to 3-4 bits for storage and comparison
- Inner-product comparisons between compressed vectors are faster
  (less data to move, potential for SIMD 4-bit ops)
- Decision cache (E16) stores compressed feature vectors: 384 dims
  × 3 bits = 144 bytes vs 384 × 4 bytes = 1.5KB (10× smaller cache)

**Router impact:** Semantic features become practical for the bandit
(currently deferred). Decision cache fits more entries in memory.
Speculation accuracy (E15) improves with richer features. The
Orchestrator's category index (E24 Part E) is more compact.

### 2.4 Gold Corpus Similarity Computation

**Where:** E21 defines a GoldSimilarityEstimator that compares local
model responses to gold references (external provider responses)
using similarity metrics. This requires embedding both responses and
computing similarity.

**TurboQuant opportunity:** Gold references are stored in OpenViking.
If response embeddings are TurboQuant-compressed:

- Gold corpus storage shrinks 10×
- Similarity computation is faster (compressed inner products)
- More gold references can be retained before staleness pruning
- Offline model evaluation (E21 Part E: corpus replay) runs faster

**Router impact:** The gold corpus flywheel (described in the
Orchestrator research doc) spins faster — more references, cheaper
comparison, more frequent recalibration.

### 2.5 Cross-Provider Context Injection Compression

**Where:** Context memory (context-memory.md) injects prior turn
summaries into requests when switching providers mid-conversation.
The injection budget is token-limited (max 4096 tokens).

**TurboQuant opportunity (indirect):** TurboQuant doesn't directly
compress text tokens, but it can compress the RETRIEVAL step. When
the router calls `contextMemory.recall()`, OpenViking performs a
semantic search over all session memories. If the vector index is
TurboQuant-compressed, this search is faster and can scan more
memories, improving the relevance of recalled context.

**Router impact:** Better context recall → better provider transition
quality (E18) → better session continuity → higher session-arc
quality (E30).

---

## 3. Novel Applications Beyond Standard Use Cases

### 3.1 Bandit Feature Space Compression

The VW contextual bandit (Phase 2, E03) uses 10 features per request.
If semantic embeddings are added (Tier 4), this grows to 10 + 384 =
394 features. VW can handle this, but the DecisionContextStore ring
buffer stores these features for every decision outcome.

With TurboQuant compression of the embedding portion:

- Ring buffer per-entry: 10 scalars + 144 bytes (compressed) vs
  10 scalars + 1.5KB (uncompressed)
- Ring buffer capacity at fixed memory: ~10× more entries for the
  embedding portion
- More history → better warm-start → faster convergence for new
  models

### 3.2 Consensus Probe Result Compression

E12 (consensus bootstrap) and E17 (speculative consensus) generate
multi-model response comparisons. Each comparison includes response
embeddings for similarity computation. TurboQuant compression of
these embeddings reduces storage and enables the Orchestrator to
retain more probe history.

### 3.3 Hyper-Category Context Filter Embeddings

E34-E36 (hyper-categories) require the context filter to determine
which memories are relevant to each role. This requires embedding-
based similarity between the current request and stored memories.
Compressed embeddings make this filtering faster, supporting the
per-role context firewalling without latency penalties.

---

## 4. Implementation Strategy

### 4.1 Where to apply first (ROI ranking)

| Integration point | Effort | Impact | ROI |
|-------------------|--------|--------|-----|
| OpenViking vector index | Medium (custom index) | High (10× storage, faster recall) | **Best** |
| Feature pipeline Tier 4 | Low (compress after embed) | Medium (richer features for bandit) | **Good** |
| Gold corpus similarity | Low (embeddings already stored) | Medium (faster recalibration) | **Good** |
| Decision cache compression | Low (compress feature vectors) | Medium (larger cache) | Good |
| LM Studio KV cache | High (backend changes) | High (longer context, more models) | Medium (depends on LM Studio) |
| Cross-provider context retrieval | Low (via OpenViking improvement) | Low-Medium | Indirect |

### 4.2 Implementation phases

**Phase 1 — Validate fundamentals (E37):**
Confirm TurboQuant's claimed accuracy on OUR embeddings (text-
embedding-3-large, all-MiniLM-L6-v2) and OUR data distribution
(routing prompts, conversation memories, code snippets). The deep
research report notes a reproducibility gap (blog claims 6× savings,
independent benchmarks show ~30% in some settings). We must verify
on our stack before building on it.

**Phase 2 — OpenViking vector compression (E38):**
Integrate TurboQuant into the vector index layer. Measure recall
quality, retrieval latency, and storage savings on real OpenViking
workloads.

**Phase 3 — Router-wide integration (E39):**
Apply TurboQuant across all integration points: feature pipeline,
decision cache, gold corpus, and (if possible) LM Studio KV cache.
Measure end-to-end routing quality impact.

---

## 5. Risks

### 5.1 Reproducibility gap

The deep research report identifies a significant gap: Google's blog
claims 6× memory reduction and 8× speedup, but independent
benchmarks (0xSero GitHub) report ~30% savings and ~5-8% throughput
improvement. Our experiments must measure actual savings on our
hardware and workload, not assume the blog numbers.

### 5.2 Embedding model sensitivity

TurboQuant's theory assumes high-dimensional Gaussian-like
distributions after rotation. Text-embedding-3-large (3072-d) likely
satisfies this (high dimension, well-distributed). all-MiniLM-L6-v2
(384-d) is lower-dimensional and may not compress as well. Must test
both.

### 5.3 Integration complexity

OpenViking's vector index is internal infrastructure. Modifying it
requires understanding OpenViking's storage format, index structure,
and search algorithm. A wrapper approach (compress/decompress at the
API boundary) is safer but may negate some latency benefits.

### 5.4 Two-stage complexity

TurboQuant requires random rotations (matrix multiplication) and
a QJL sketch (random projection + sign). These operations need
careful GPU implementation. On CPU (where OpenViking likely runs
its search), the speedup may be minimal or negative — the
decompression overhead could exceed the bandwidth savings.

### 5.5 LM Studio dependency

KV cache compression in LM Studio requires changes to the inference
backend. LM Studio is third-party software; we can't modify it
directly. vLLM with TurboQuant kernels is an alternative but would
require migrating from LM Studio — a major infrastructure change.
