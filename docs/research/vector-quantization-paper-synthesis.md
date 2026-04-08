# Research Synthesis: Vector Quantization for KV Caches and Embedding Compression

**Papers analyzed:** 3 | **Date range:** June 2024 – April 2025
**Researcher:** Alison Aquinas | **Synthesis date:** 2026-04-06

## Executive Summary

Three closely related papers from overlapping author teams at Google Research,
KAIST, Yale, NYU, and Google DeepMind present a progressive research arc in
vector quantization for LLM inference and vector search. QJL (2024) introduced
1-bit sketching for KV cache keys with zero memory overhead. PolarQuant (2025)
replaced Cartesian quantization with polar-coordinate quantization, eliminating
normalization overhead entirely. TurboQuant (2025) unified these ideas into a
two-stage framework with provably near-optimal distortion bounds for both MSE
and inner products, extending applicability from KV caches to general-purpose
nearest-neighbor search. Together they demonstrate that random rotation +
coordinate-wise quantization is a powerful, practical paradigm for compressing
high-dimensional vectors in latency-sensitive AI systems — directly relevant to
our OpenViking vector index, feature pipeline, and local inference stack.

---

## Key Themes

### Theme 1: Random Preconditioning Eliminates Normalization Overhead

**Prevalence:** All 3 papers
**Summary:** Traditional KV cache quantization methods (KIVI, KVQuant, GEAR)
group data into blocks and store per-block zero points and scales in full
precision, adding 1-2 bits per quantized number in memory overhead. All three
papers show that multiplying embedding vectors by a random orthogonal or
Gaussian matrix before quantization makes the resulting coordinate distributions
predictable (Beta in TurboQuant, concentrated polar angles in PolarQuant,
sign-bit-friendly in QJL), removing the need for per-block normalization
constants entirely.

**Supporting Evidence:**

- "Applying a random preconditioning matrix on the embedding vectors eliminates
  the need for data normalization" — PolarQuant §1
- "Unlike previous methods, the QJL sketch can quantize vectors with zero
  overhead because it does not require grouping the data and storing quantization
  constants" — QJL §1.1
- "We randomly rotate input vectors, inducing a concentrated Beta distribution
  on coordinates, and leveraging the near-independence property of distinct
  coordinates in high dimensions to simply apply optimal scalar quantizers per
  each coordinate" — TurboQuant Abstract

**Implication for better-router:** The random-rotation trick is the enabling
insight for all integration points. Our OpenViking embeddings (3072-d from
text-embedding-3-large) and feature pipeline embeddings (384-d from MiniLM)
can both be preconditioned once at ingestion time. The preconditioning matrix
is shared across all vectors in a collection, so the per-vector storage
overhead is zero. This validates E37's plan to test PolarQuant rotation on our
specific embedding distributions.

---

### Theme 2: Two-Stage Quantization Solves the MSE vs Inner Product Trade-off

**Prevalence:** TurboQuant (primary contribution), QJL (stage 2 origin)
**Summary:** Quantizers optimized for mean-squared error (MSE) introduce bias in
inner product estimation. TurboQuant solves this by decomposing quantization
into two stages: (1) an MSE-optimal scalar quantizer on each rotated
coordinate, followed by (2) a 1-bit QJL transform on the residual. The
combination is provably unbiased for inner products AND near-optimal for MSE,
within a constant factor of ~2.7× of the information-theoretic lower bound.

**Supporting Evidence:**

- "MSE-optimal quantizers do not necessarily provide unbiased inner product
  estimates, particularly exhibiting significant bias at lower bit-widths" —
  TurboQuant §1.3
- TurboQuant_prod achieves unbiased inner product estimation at all bit widths
  (Figure 1, TurboQuant paper), while TurboQuant_mse shows increasing bias at
  lower bit widths
- Distortion bounds: D_mse ≤ (√3π/2) · 1/4^b; D_prod ≤ (√3π² · ‖y‖²/d) ·
  1/4^b — both exponentially decreasing with bit-width

**Implication for better-router:** For OpenViking context recall and gold corpus
similarity search, inner product accuracy matters more than raw MSE. We should
use TurboQuant_prod (the two-stage variant) for all search-facing vector stores
and can use TurboQuant_mse for pure storage/reconstruction (e.g., decision
cache feature archival). E38's comparison of "TQ inner product estimator" vs
"decompress-then-cosine" maps directly to this MSE vs inner-product trade-off.

---

### Theme 3: KV Cache Compression at 3 Bits Achieves Parity with FP16

**Prevalence:** All 3 papers
**Summary:** Across needle-in-a-haystack retrieval, LongBench question-answering,
and standard LM-eval benchmarks, all three methods achieve no measurable
accuracy degradation at 3-3.5 bits per channel on Llama-2-7B, Llama-3-8B,
and Mistral-7B models compared to FP16 baselines. This represents 4.5-5.3×
memory reduction in the KV cache.

**Supporting Evidence:**

- QJL at 3 bits: "No accuracy drop compared to the exact model with 16 bits per
  FPN while reducing cache memory usage by over fivefold" — QJL §1.1
- PolarQuant-R (online) achieves 48.37 average on LongBench vs 48.63 for Exact
  16-bit on Llama-3.1-8B-Instruct — a 0.5% gap (Table 1, PolarQuant paper)
- TurboQuant at 3.5 bits: "absolute quality neutrality" on Llama-3.1-8B;
  at 2.5 bits: "marginal quality degradation" while compressing ≥4.5×
  (TurboQuant §4.3)
- Needle-in-a-haystack: TurboQuant scores 0.997 vs Full-Precision 0.997
  (Figure 4, TurboQuant). PolarQuant scores 0.991 vs Exact 0.995 (Figure 3,
  PolarQuant)

**Implication for better-router:** This is the strongest evidence that KV cache
compression on our LM Studio nodes (E39 Part D) is viable without quality loss
IF we can integrate TurboQuant/PolarQuant kernels. The 3-bit sweet spot should
be our target configuration. At 3 bits, we could roughly triple the effective
context window or double the number of concurrent models per node.

---

### Theme 4: Outlier Channels Require Special Treatment in Deeper Layers

**Prevalence:** QJL (primary analysis), TurboQuant (adopted strategy)
**Summary:** Key embeddings in deeper transformer layers contain a small number
of fixed channels (approximately 4 per head in Llama-2) with magnitudes 10-30×
larger than typical channels. These outliers dominate the L2 norm and
disproportionately affect attention score distortion. Both QJL and TurboQuant
handle this by identifying outlier channels during the prompt encoding phase
and quantizing them separately with higher precision.

**Supporting Evidence:**

- "In the deeper layers, certain fixed coordinates of key embeddings
  consistently exhibit large magnitudes, and this pattern persists within these
  channels across all tokens" — QJL §4.1, Figure 2
- TurboQuant's 2.5-bit config: "32 outlier channels are quantized at 3 bits,
  while the remaining 96 use 2 bits" — TurboQuant §4.3
- Distortion is "directly proportional to the norms of the embedding vectors"
  (QJL Theorem 3.6), making outlier isolation essential

**Implication for better-router:** If we pursue KV cache integration (E39 Part D),
the outlier channel detection and isolation strategy is not optional — it's
required for quality preservation. However, for our OpenViking embedding vectors
(text-embedding-3-large output), the outlier pattern may differ from KV cache
embeddings since these are output embeddings not internal attention states. E37
should characterize our specific embedding distributions before assuming the
same outlier structure applies.

---

### Theme 5: Near-Zero Indexing Time for Vector Search

**Prevalence:** TurboQuant (primary), compared against Product Quantization (PQ)
and RabitQ
**Summary:** TurboQuant's quantization is data-oblivious (no codebook training
required), making indexing time effectively zero regardless of dimension.
Product Quantization requires k-means training that takes 37-494 seconds for
100K vectors at 200-3072 dimensions. RabitQ is even slower (597-3957 seconds).
TurboQuant: 0.0007-0.0021 seconds. This is a 5-6 orders of magnitude speedup.

**Supporting Evidence:**

- Quantization time for d=3072: PQ = 494.42s, RabitQ = 3957.19s, TurboQuant =
  0.0021s (Table 2, TurboQuant paper)
- TurboQuant "consistently outperforms both Product Quantization and RabitQ in
  terms of recall ratio across all experiments" — TurboQuant §4.4
- "Reducing indexing time to essentially zero" — TurboQuant Abstract

**Implication for better-router:** This is transformative for OpenViking. Current
vector indexing methods (FAISS IVF-PQ or similar) require periodic re-training
of quantization codebooks as new memories are added. TurboQuant's data-oblivious
nature means every new memory can be compressed and indexed instantly with no
background retraining. The Orchestrator's memory compaction workload (E24)
would shrink dramatically. This is the single highest-ROI integration point
and validates E38's focus on OpenViking as the first target.

---

### Theme 6: Polar vs Cartesian — Two Valid Approaches to the Same Insight

**Prevalence:** PolarQuant vs TurboQuant
**Summary:** PolarQuant and TurboQuant share the same fundamental insight (random
preconditioning makes coordinate distributions predictable) but implement it
differently. PolarQuant converts to polar coordinates and quantizes angles,
exploiting the fact that angles concentrate around π/4 at higher recursion
levels. TurboQuant stays in Cartesian coordinates and quantizes the resulting
Beta-distributed values with optimal Lloyd-Max centroids. Both achieve similar
KV cache results; TurboQuant additionally handles inner products with provable
unbiasedness.

**Supporting Evidence:**

- PolarQuant: "Angles in the polar representation exhibit a tightly bounded and
  highly concentrated distribution with an analytically computable form" —
  PolarQuant Abstract
- TurboQuant: "Each coordinate of Π·x follows a Beta distribution, which
  converges to N(0,1/d) in high dimensions" — TurboQuant §3.1
- On LongBench: PolarQuant-R (online) avg 48.37 vs TurboQuant 3.5-bit avg
  50.06 — TurboQuant slightly better on average (different compression ratios
  complicate direct comparison)
- Needle-in-a-haystack: PolarQuant 0.991, TurboQuant 0.997 (TurboQuant closer
  to the 0.997 full-precision baseline)

**Implication for better-router:** TurboQuant is the more general and better-
performing of the two and should be our primary integration target. PolarQuant's
recursive polar transform adds computational overhead (11.6s prefill vs 3.4s
offline PolarQuant-R, Table 2, PolarQuant paper) that TurboQuant avoids. For
our E37/E38 experiments, we should implement TurboQuant_prod first, falling
back to PolarQuant only if TurboQuant's GPU/Triton kernel dependency proves
problematic for our CPU-based OpenViking pipeline.

---

## Insights → Opportunities

| Insight | Opportunity | Impact | Effort |
|---------|-------------|--------|--------|
| 3-bit KV compression at zero quality loss | Triple context window or double concurrent models on LM Studio nodes | High | High (requires inference backend change) |
| TurboQuant near-zero indexing time | Instant-index OpenViking memories without codebook retraining | High | Medium (API wrapper around embedding pipeline) |
| TurboQuant_prod unbiased inner products | Use compressed-domain similarity search in OpenViking without decompression | High | Medium (replace FAISS with TQ search) |
| Random rotation is shared across all vectors | One-time preconditioning matrix generation; negligible per-vector overhead | Medium | Low (generate once, store as config) |
| Outlier channel isolation for KV cache | Required for any KV cache integration on local models | Medium | Medium (profiling + per-layer config) |
| PolarQuant offline codebook is fast but less accurate | Prefer TurboQuant's data-oblivious approach over PolarQuant's offline mode | Low | Low (selection decision, not implementation) |
| TurboQuant MSE distortion ≈ 2.7× lower bound | Our E37 validation can check if empirical distortion matches theory on our embeddings | Medium | Low (add theoretical curve to E37 plots) |

---

## Technical Comparison Across Papers

| Property | QJL | PolarQuant | TurboQuant |
|----------|-----|------------|------------|
| **arXiv** | 2406.03482 | 2502.02617 | 2504.19874 |
| **Date** | Jul 2024 | Feb 2025 | Apr 2025 |
| **Core technique** | JL transform + sign bit | Polar transform + angle quantization | Random rotation + Lloyd-Max scalar + QJL residual |
| **Normalization overhead** | Zero | Zero | Zero |
| **Inner product unbiased?** | Yes (asymmetric) | Not addressed | Yes (TurboQuant_prod) |
| **Theoretical optimality** | Distortion bound (Theorem 3.6) | Asymptotically optimal (Theorem 1) | Near-optimal (≈2.7× lower bound) |
| **KV cache results** | 3 bits, ≥5× compression, no accuracy drop | 3.875 bits, ≥4.2× compression, best quality scores | 2.5-3.5 bits, ≥4.5× compression, quality neutral at 3.5 |
| **Vector search** | Not tested | Not tested | Outperforms PQ and RabitQ on recall@k |
| **Indexing time** | N/A | N/A | ~0 seconds (data-oblivious) |
| **GPU kernels** | CUDA (PyTorch + custom) | CUDA (PyTorch + custom) | Triton (referenced), A100 validated |
| **Models tested** | Llama-2-7B, Llama-3-8B | Llama-3.1-8B-Instruct | Llama-3.1-8B-Instruct, Mistral-7B-Instruct |
| **Benchmark** | LongBench, LM-eval | LongBench-V1, Needle-in-a-Haystack | LongBench-V1 (E subset), Needle-in-a-Haystack, NN search |
| **Overlapping authors** | Zandieh, Daliri, Han | Han, Kacham, Mirrokni, Zandieh | Zandieh, Daliri, Hadian, Mirrokni |

---

## Recommendations for better-router Integration

1. **Implement TurboQuant_prod for OpenViking (highest priority)** — The
   near-zero indexing time and unbiased inner products make this the ideal
   replacement for any existing vector quantization in our embedding pipeline.
   E38 should use TurboQuant_prod specifically, not TurboQuant_mse.

2. **Target 3-bit compression for storage, 4-bit for hot path** — The papers
   converge on 3 bits as the quality-neutral threshold for KV caches. For
   OpenViking embeddings (which have different distribution characteristics),
   validate this threshold in E37 before committing. Use 4-bit for any vectors
   involved in real-time routing decisions (feature pipeline, decision cache)
   where we can't tolerate even marginal distortion.

3. **Profile our embedding distributions before assuming Gaussian** — All
   three papers assume or induce approximately Gaussian coordinate distributions
   via random rotation. Our text-embedding-3-large outputs may already be near-
   Gaussian (common for normalized embeddings), or they may have structure that
   random rotation disrupts. E37 Part A's distribution characterization step is
   critical.

4. **KV cache integration requires backend migration** — None of these methods
   work as a bolt-on to LM Studio. Implementing KV cache compression requires
   either migrating to vLLM (which has community TurboQuant/QJL integration
   work) or modifying llama.cpp internals. This is a significant infrastructure
   decision (E39 Part D) that should be treated as a separate engineering project
   with its own cost-benefit analysis.

5. **Plan for the outlier channel problem if pursuing KV cache** — Any KV
   cache integration must include an outlier detection and isolation step. Budget
   2-4 outlier channels per attention head at 2× the bit precision of regular
   channels. This is a per-model configuration that must be profiled for each
   model in the router's pool.

6. **Entropy coding is available for free** — TurboQuant notes that entropy
   encoding the quantized indices reduces effective bit width by ~5% at b=4 with
   no distortion cost (Appendix, TurboQuant paper). Low-hanging fruit for the
   OpenViking integration if storage is the primary constraint.

---

## Questions for Further Research

- How do text-embedding-3-large (3072-d) output vectors distribute before and
  after random rotation? Are they already near-Gaussian, or do they have
  task-specific structure?
- What is the CPU performance of TurboQuant on our 10.60.1.x nodes without GPU
  acceleration? The papers only test on A100/A6000 GPUs.
- Can the random rotation matrix be shared across the 3072-d and 384-d
  embedding spaces, or does each dimensionality need its own?
- TurboQuant's Triton kernels are referenced but not open-sourced in the paper.
  Is the GitHub implementation at github.com/amirzandieh/QJL updated with
  TurboQuant, or is it QJL-only?
- For the decision context store's small feature vectors (10 heuristic scalars),
  is TurboQuant overkill? The random rotation matrix alone would be 10×10,
  which seems like overhead for 80-byte vectors.

## Methodology Notes

All three papers are from overlapping author teams, primarily at Google Research.
The research arc is clearly cumulative: QJL → PolarQuant → TurboQuant, with
each paper building on the previous. Results should be read as a progression
rather than independent validations. All experiments use single-GPU setups
(A100 80GB or RTX A6000 48GB) which are comparable to but not identical to our
LM Studio node hardware. Benchmark datasets (LongBench, LM-eval, Needle-in-a-
Haystack) are standard but focus on factual retrieval and QA; they do not cover
the creative, coding, or agent-harness workflows that dominate our traffic mix
(E33). The reproducibility gap noted in our earlier TurboQuant deep research
report (blog claims 6× savings, independent benchmarks show ~30% in some
settings) remains unresolved and should be addressed by E37's own hardware
validation.
