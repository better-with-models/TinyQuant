---
title: "Calibration Threshold Investigation: Diagnose and Fix pr_speed Failures"
tags:
  - plans
  - rust
  - calibration
  - codec
  - quality-recovery
  - regression
date-created: 2026-04-14
status: complete
category: planning
---

# Calibration Threshold Investigation

> [!info] Goal
> Establish ground truth for the codec's real PR-speed calibration
> numbers on a trusted fixture, then either (a) fix the codec, (b) fix
> the residual encoding, (c) relax the thresholds against honest
> measured values, or (d) any combination — so that the
> `rust-calibration.yml` workflow is a meaningful release gate rather
> than a permanently red ceremony.

## 1. Background

On 2026-04-14 the newly-committed `rust-calibration.yml` workflow
(`c8482fc` on `phase-21-rayon-batch`) was exercised locally for the
first time against its own `pr_speed` test group. It failed. A second
run with `--features simd`, and a third run inside a Linux
`rust:1.81` Docker container against the same worktree, all produced
**bit-identical** failure numbers. Platform drift is ruled out.

The workflow cannot have passed in CI previously: it was a brand-new
workflow, and the calibration job had also never run under its prior
home (it used to be a `matrix.calibration-*` job inside `rust-ci.yml`
that was gated behind on-demand-only triggers). So this is a
long-standing gap in the release-readiness story — not a regression
from the Phase 22–25 chain that simply exposed it by running it.

## 2. Observed facts

### 2.1 Failure matrix

Source: `rust/crates/tinyquant-bench/tests/calibration.rs` lines
159–173 (hard `assert!` against fields of the relevant `Threshold`
constant), fixture `openai_1k_d768.f32.bin` (rows = 1000, cols = 768,
raw_bytes = 3 072 000).

| Test | Threshold | Windows MSVC | Linux glibc | Δ |
|---|---|---|---|---|
| `pr_speed_bw2_residual_off_meets_thresholds` | ρ ≥ 0.85 | ρ = **0.5119** | ρ = **0.5119** | 0 |
| `pr_speed_bw4_residual_off_meets_thresholds` | ρ ≥ 0.98 | ρ = **0.9573** | ρ = **0.9573** | 0 |
| `pr_speed_bw2_residual_on_meets_thresholds`  | ratio ≥ 14 | ratio = **1.78** | ratio = **1.78** | 0 |
| `pr_speed_bw4_residual_on_meets_thresholds`  | ratio ≥ 7  | ratio = **1.60** | ratio = **1.60** | 0 |
| `pr_speed_bw8_residual_on_meets_thresholds`  | ratio ≥ 4  | ratio = **1.33** | ratio = **1.33** | 0 |

### 2.2 Two independent gaps

The failures are **two structurally different problems**, not one:

**Gap A — residual-on compression ratio.** The observed ratios match
the closed-form expression for "quantized bytes + uncompressed fp16
residual":

```
ratio = raw_bytes / (quantized_bytes + 2·dim·rows)
```

- bw = 8: 3 072 000 / (1·768·1000 + 2·768·1000) = 1.333 ✓
- bw = 4: 3 072 000 / (0.5·768·1000 + 2·768·1000) = 1.600 ✓
- bw = 2: 3 072 000 / (0.25·768·1000 + 2·768·1000) = 1.778 ✓

Interpretation: the codec is shipping **raw fp16 residuals**, not a
further-compressed residual payload. The 4×–14× thresholds assume a
residual encoding that currently doesn't exist in the code path.

**Gap B — residual-off Pearson ρ.** Quantization-only reconstruction
quality is below threshold:

- bw = 2 residual=off: ρ = 0.5119 (threshold 0.85, miss by 40%)
- bw = 4 residual=off: ρ = 0.9573 (threshold 0.98, miss by 2.5%)

ρ = 0.51 for 2-bit PQ on a 768-dim OpenAI embedding corpus is
unusually low even accounting for bw=2's aggressive lossiness. The
2.5% miss at bw=4 might be within tuning noise, but the bw=2 miss is
large enough to suggest a structural issue (missing rotation,
degenerate codebook, or similar) rather than ordinary tuning.

### 2.3 What is not the cause

Ruled out by the evidence we have:

| Hypothesis | Status | Evidence |
|---|---|---|
| Platform (Windows vs Linux) | Ruled out | Bit-identical numbers on both. |
| `--features simd` vs scalar | Ruled out | Bit-identical numbers on both feature sets. |
| Fixture corruption | Ruled out | `sha256sum` matches `manifest.json` on both files. |
| Toolchain drift | Ruled out | Both runs use rust 1.81.0; manifests match the pinned toolchain. |
| Rayon batch path | Unknown | Not ruled out. Phase 21's parallel path is used under `std`; serial path under `no_std`. Determinism observed doesn't exclude an incorrect-but-deterministic regression. |
| Thumbv7em toolchain state | Ruled out as cause of test failure | The sync conflict affected the zeroth run only; resolved by re-installing the target. |

## 3. Hypothesis tree

> Priors expressed as {Low, Medium, High} — prior probability that
> this hypothesis is the primary cause. **Posteriors (2026-04-14)**
> are in bold after each hypothesis; see §7 ledger and §12 work log
> for supporting evidence.

**Posterior summary.** Both hypotheses H1 (aspirational thresholds)
and H2 (missing residual encoder) are **confirmed** as co-causes.
H3, H4, H5 are **ruled out**.

- **H1 → confirmed** (Gap B, rho/recall): thresholds are plan-doc
  targets, not measurements. A3+A4 found the commit that "restored
  spec-mandated thresholds" (59af898) while noting the plan's own
  rule (tighter thresholds require matching goals-doc raise) was
  itself violated in the same commit.
- **H2 → confirmed** (Gap A, ratio): A5 confirmed `CompressedVector`
  stores raw `f16` residuals with honest `size_bytes()` accounting;
  ratio is structurally capped at `4 / (bw/8 + 2)` regardless of
  bit width. A2 confirmed the Python reference oracle exhibits the
  same cap, so the missing encoder is an architecture gap, not a
  Rust regression.
- **H3 → ruled out**: A2 Python reference produces rho within 0.022
  of Rust at bw=2 residual=off and within 0.002 at bw=4 residual=off.
  Both impls are on the scalar-quantizer ceiling for this fixture.
- **H4 → ruled out**: A7 fixture stats show 1000 unique rows,
  zero zero-std dims, per-dim std tightly clustered, unit norms.
  Fixture is synthetic isotropic-sphere Gaussian (per A7 caveat)
  rather than real OpenAI — but this is the *best* case for a
  scalar quantizer, not a degeneracy.
- **H5 → ruled out**: A6 replay at 305c921 (pre-Phase-21, serial
  `compress_batch`) produces bit-identical rho/ratio to current
  HEAD across all 5 configs. A2 independently confirms via Python
  oracle (identical numbers would not result from a Phase-21
  perturbation).

### H1 — Thresholds were set aspirationally (prior: Medium → **posterior: HIGH, confirmed**)

The `Threshold` constants in `calibration.rs:32-56` reference
"§Calibration thresholds" (a plan doc section, not a measurement
record). The thresholds may have been written against a design spec
that was never fully implemented.

### H2 — Residual encoding is incomplete (prior: High for Gap A → **posterior: HIGH, confirmed**)

The formula match in §2.2 is mathematically exact to four decimal
places across three `bw` values. This is not coincidence — it is the
strongest signal in the dataset. Either (a) residual compression was
designed but never implemented, or (b) `CompressedVector::size_bytes`
is counting the in-memory buffer rather than the serialized form.

### H3 — ~~Quantization quality regression~~ (prior: Medium for Gap B → **posterior: LOW, ruled out**)

A codebook/rotation/quantize regression could lower ρ systematically.
Candidate points of failure:

- **H3a**: rotation matrix is not applied in the codec hot path.
- **H3b**: `Codebook::train` produces sub-optimal quantile breakpoints.
- **H3c**: quantize/dequantize has an off-by-one (e.g., treats signed
  codes as unsigned, shifts wrong direction).
- **H3d**: `Codec::new()` is a passthrough factory and the caller
  expected `Codec::with_rotation()` or similar.
- **H3e**: the `rand_chacha` seed path changed between the time
  thresholds were authored and now.

### H4 — ~~Fixture is misaligned~~ (prior: Low → **posterior: LOW, ruled out** at byte level; realism caveat noted)

SHAs match `manifest.json`, so the binary is what was committed. But
the binary could still be degenerate for the codec's expectations
(e.g., vectors drawn from a different OpenAI model than the thresholds
assumed). Low prior but cheap to check.

### H5 — ~~Phase 21 rayon-batch introduced a subtle numeric perturbation~~ (prior: Low → **posterior: LOW, ruled out** via A6 replay + A2 Python cross-check)

Phase 21 added `compress_batch_parallel` under the `std` feature. If
the parallel reduction order differs from the serial reference, we'd
see tiny numeric drift — but not a 40% Pearson gap. Low prior, but
cheap to rule out with a pre-Phase-21 replay.

## 4. Phase A — diagnostic sweep

Goal: narrow the hypothesis space before committing to a remediation
path. Each step has a **Command**, an **Expected signal**, and a
**Decision gate** that determines whether to continue.

### A1. Print all three metrics (independent of threshold)

**Why first:** we currently see only the first failing metric because
`run_gate` short-circuits on the first violated assertion. We don't
know the actual `recall@10`, we don't know whether the ratio fails
when ρ also fails, we don't know if any of them pass silently.

**Command:** patch `rust/crates/tinyquant-bench/tests/calibration.rs`
locally (do not commit) to emit all three metrics before asserting:

```rust
fn run_gate(corpus: GoldCorpus, bw: u8, residual: bool, threshold: &Threshold) {
    // ...existing body through `let (rho, recall, ratio) = score(...);`
    eprintln!(
        "METRIC bw={bw} residual={residual} rho={rho:.4} recall@10={recall:.4} ratio={ratio:.4}"
    );
    // ...existing assertions unchanged
}
```

Run with `--features simd` and `--nocapture`:

```bash
cargo test --release -p tinyquant-bench --features simd \
  -- --ignored pr_speed --nocapture
```

**Expected signal:** full `(ρ, recall, ratio)` for all 5 pr_speed
tests plus any full-bw* / core-* groups that are enabled.

**Decision gate:**
- If **recall** also misses its thresholds widely → Gap B is
  system-wide, not just ρ. Prioritize H3 over H1.
- If **recall passes** even when ρ fails → the codec produces
  locally-useful reconstructions but scrambles global correlations.
  Points at rotation or whitening.
- Record the full numeric table in §7 of this doc before moving on.

### A2. Cross-reference against the Python reference

**Why:** `tests/reference/tinyquant_py_reference/` is declared in
`CLAUDE.md` as the "behavioral gold standard" for the Rust port. If
the Python reference clears the thresholds, the Rust codec has a
regression. If the Python reference also misses, the thresholds are
aspirational.

**Command:**

```bash
cd C:/Users/aaqui/better-with-models/TinyQuant/.worktrees/phase-22-25-release-chain
python - <<'PY'
import numpy as np
from tests.reference.tinyquant_py_reference import Codec, Codebook, CodecConfig

vectors = np.fromfile(
    "rust/crates/tinyquant-bench/fixtures/calibration/openai_1k_d768.f32.bin",
    dtype=np.float32,
).reshape(1000, 768)

for bw, residual in [(2, False), (2, True), (4, False), (4, True), (8, True)]:
    cfg = CodecConfig(bw=bw, seed=42, dim=768, residual=residual)
    cb = Codebook.train(vectors, cfg)
    codec = Codec()
    cvs = [codec.compress(v, cfg, cb) for v in vectors]
    recon = np.stack([codec.decompress(c, cfg, cb) for c in cvs])
    # Pearson on 500 cosine-pair samples (match Rust score() semantics)
    # ... (use the algorithm from calibration.rs:76-133)
    raw_bytes = vectors.nbytes
    compressed_bytes = sum(cv.size_bytes() for cv in cvs)
    ratio = raw_bytes / compressed_bytes
    # ... compute rho, recall same way Rust does
    print(f"PY bw={bw} residual={residual} rho=... recall=... ratio={ratio:.4f}")
PY
```

**Expected signal:** a table of Python (ρ, recall, ratio) for the
same five configurations. The `score()` sampling algorithm in
`calibration.rs:76-132` must be ported byte-for-byte for the numbers
to be comparable.

**Decision gate:**
- Python (ρ, ratio) ≈ Rust (ρ, ratio) → algorithm is consistent.
  Thresholds are aspirational. Weight H1 heavily.
- Python ρ ≥ thresholds, Rust ρ < thresholds → Rust regression.
  Weight H3 heavily.
- Python also produces ratio ≈ 1.3–1.8 on residual=on → residual
  encoding is structurally the same in both impls. Weight H2 heavily.
- Python residual=on ratio ≥ 4 but Rust is 1.3 → Rust is missing a
  residual compression step that the Python reference has.

### A3. Git archaeology on the threshold table

**Why:** thresholds may have a PR / commit that documents their
origin ("spec lower bound per Phase 21 plan").

**Command:**

```bash
cd C:/Users/aaqui/better-with-models/TinyQuant/.worktrees/phase-22-25-release-chain

# When was calibration.rs first authored?
git log --follow --format="%h %ad %s" --date=short \
  -- rust/crates/tinyquant-bench/tests/calibration.rs | tail -20

# When were the threshold constants last touched?
git log -p --follow -- rust/crates/tinyquant-bench/tests/calibration.rs \
  | grep -E "^(commit |diff --git|\+.*Threshold|\+.*rho_min|\+.*ratio_min)" \
  | head -40

# Did the thresholds ever pass in CI? Search workflow runs.
# (Requires network; defer to someone with gh CLI access to the repo.)
```

**Expected signal:** the commit that added the thresholds and the PR
description that justifies them.

**Decision gate:**
- Thresholds cite a design doc (e.g. Phase 21 plan §Calibration
  thresholds) → read that doc in A4.
- Thresholds have no explanation in commit history → they are
  measured against a now-missing earlier codec state or were
  aspirational. Weight H1 heavily.

### A4. Plan-doc provenance

**Why:** the threshold constants reference "§Calibration thresholds"
as the spec. Locate that section and compare prose to code.

**Command:**

```bash
# Find any doc that defines calibration thresholds.
grep -rn "rho_min\|recall_at_10_min\|ratio_min\|Calibration thresholds" \
  docs/plans/rust docs/design/rust 2>/dev/null | head -30
```

**Expected signal:** a markdown table of planned (ρ, recall, ratio)
values with context about how they were derived.

**Decision gate:**
- Plan doc says "measured against reference implementation" + cites a
  specific Python reference version → cross-check whether that
  reference can reproduce them (A2 covers this).
- Plan doc says "aspirational / desired" without measurement → H1
  confirmed. Move to Phase B1.
- Plan doc says "from published paper / library X" → check whether
  we implement the same algorithm as X (we may have diverged).

### A5. Residual encoding contract audit

**Why:** confirms whether Gap A is "missing encoder" or
"size_bytes accounting bug".

**Command:**

```bash
# What does size_bytes compute? Follow the impl.
grep -rn "fn size_bytes\|size_bytes(" rust/crates/tinyquant-core/src/

# What does the on-disk / wire format encoder do with the residual?
# Look for serialization of CompressedVector.
grep -rn "serialize\|encode\|write_to\|to_bytes" \
  rust/crates/tinyquant-core/src/codec/ rust/crates/tinyquant-io/src/
```

Then compress 10 vectors by hand, dump the residual field, compare
its byte layout to `2·dim` fp16:

```rust
// Scratch test, do not commit:
#[test]
fn residual_layout_audit() {
    let corpus = [0.1_f32; 1000 * 768];  // or load fixture
    let cfg = CodecConfig::new(4, 42, 768, true).unwrap();
    let cb = Codebook::train(&corpus[..10 * 768], &cfg).unwrap();
    let codec = Codec::new();
    let cv = codec.compress(&corpus[..768], &cfg, &cb).unwrap();
    eprintln!("cv.size_bytes() = {}", cv.size_bytes());
    // Dump residual field — is it Vec<u16> (raw fp16) or Vec<u8>
    // (compressed)?  Inspect via Debug or a getter.
}
```

**Expected signal:** whether the residual field is raw fp16 or some
compressed form.

**Decision gate:**
- Residual is raw fp16, `size_bytes` counts it honestly → H2 confirmed
  as "missing encoder". Plan B2.
- Residual is raw fp16 but `size_bytes` counts something smaller →
  accounting inconsistency. Cheap fix in B1 (thresholds) or B2
  (real encoder).
- Residual is already compressed but ratio is still 1.3 → deeper
  bug. Escalate to B3.

### A6. Replay on pre-Phase-21 HEAD

**Why:** rules out H5 (Phase 21 perturbation) cheaply.

**Command:** use a sibling worktree (do not disturb the release
chain). Target: the commit just before Phase 21 merged (git log for
"phase-21" keyword).

```bash
cd C:/Users/aaqui/better-with-models/TinyQuant
# Find the parent of the first phase-21 commit on the main line.
git log --oneline --grep="phase-21" --reverse | head -5
# Replay pr-speed there in a sibling worktree.
git worktree add ../TinyQuant-phase20-replay <pre-phase-21-sha>
cd ../TinyQuant-phase20-replay/rust
cargo test --release -p tinyquant-bench --features simd -- --ignored pr_speed
```

**Expected signal:** ρ and ratio on the pre-Phase-21 codec.

**Decision gate:**
- ρ, ratio identical to current HEAD → Phase 21 is not the cause.
  Drop H5.
- ρ noticeably higher pre-Phase-21 (e.g. 0.85+) → Phase 21 is the
  regression point. Bisect commits in B5.

### A7. Degenerate-fixture sanity

**Why:** cheap rule-out for H4b (corpus doesn't suit codec's expected
distribution).

**Command:**

```bash
python - <<'PY'
import numpy as np
v = np.fromfile(
    "rust/crates/tinyquant-bench/fixtures/calibration/openai_1k_d768.f32.bin",
    dtype=np.float32,
).reshape(1000, 768)
print(f"shape={v.shape}")
print(f"mean={v.mean():.4f}  std={v.std():.4f}")
print(f"per-row norm range: {np.linalg.norm(v, axis=1).min():.4f} "
      f"..  {np.linalg.norm(v, axis=1).max():.4f}")
# Detect all-zero rows, duplicate rows, or near-constant dims.
print(f"unique rows: {np.unique(v, axis=0).shape[0]}")
print(f"per-dim std min/max: {v.std(axis=0).min():.6f} "
      f".. {v.std(axis=0).max():.6f}")
PY
```

**Expected signal:** well-distributed fp32 vectors with typical
OpenAI-embedding stats (unit-norm, near-zero-mean, no degenerate
dims).

**Decision gate:**
- Stats look normal for OpenAI embeddings → H4 ruled out. Move on.
- Stats are degenerate (many zero-std dims, duplicate rows, wild norm
  spread) → fixture is bad. Regenerate from a clean seed.

## 5. Phase B — remediation branches

One branch per verdict. Phase A output selects which branch (or
combination) applies. Branches are cumulative: B1 can coexist with B3,
for example.

### B1 — Relax thresholds against honest measurement

**When:** H1 confirmed (thresholds were aspirational) OR as a short-
term mitigation while B2/B3 are in flight.

**Design:**

1. Replace the `Threshold` constants with values derived from the
   Phase-A1 measurement table, rounded down to two decimals with a
   small safety margin (e.g., measured 0.9573 → threshold 0.95).
2. Add a `// TODO(phase-26):` comment on each relaxed constant
   citing this investigation doc.
3. Add a new entry **R20 — Interim calibration thresholds** to
   `docs/design/rust/risks-and-mitigations.md` with:
   - observed vs aspirational values
   - date of measurement
   - trigger for re-tightening (e.g., "once B2 ships, raise ratios
     back to 4/7/14")

**Tests:** the existing 5 tests must pass with new constants on a
clean clone of `phase-22-25-release-chain`, with and without
`--features simd`, on Linux x86_64 and Windows x86_64. No other tests
should change.

**Docs:**

- Add a §note in `tests/calibration.rs:22-30` explaining that the
  thresholds were re-baselined on 2026-04-… against measured values.
- Update `CHANGELOG.md` under `[Unreleased]` with a
  `### Calibration` subsection noting the baseline refresh.
- Do NOT flip any phase doc's `status` from `complete` to something
  else — this is not a phase rollback, it's an interim calibration.

**Exit criterion:** `rust-calibration.yml` passes on its first CI run
after merge.

### B2 — Implement residual compression

**When:** H2 confirmed (encoder missing) AND the plan-doc provenance
in A4 establishes that residual compression was in scope.

**Out of this plan's scope:** B2 is a design-level change and should
be broken out into its own phase plan (likely **Phase 26 —
Residual compression**). This doc should produce a three-paragraph
motivation that a future Phase 26 plan can cite.

**Sketch only (do not implement from this plan):**

- Candidate encodings: sparse-nonzero (threshold + index), low-rank
  projection, or entropy-coded (rANS / range coding). Each has a
  different ratio / quality trade-off.
- A typical "free" improvement is to drop residual values below a
  threshold, encoding only the significant ones. This can lift
  ratios from ~1.3 to ~3–5 without measurable ρ loss.
- The encoder must ship alongside a decoder, a fixture-parity snapshot
  test, and a deterministic byte format pinned by `#[repr(C)]` or
  serde + bincode with a version field.

**Exit criterion (Phase 26 scope):** residual-on ratios clear the
original aspirational thresholds (bw2 ≥ 14, bw4 ≥ 7, bw8 ≥ 4) while
keeping ρ within 0.5% of the residual-off baseline.

### B3 — Quantization quality repair

**When:** H3 confirmed (ρ regression, not tuning).

**Diagnostic sub-steps:**

1. Bisect `git log --oneline rust/crates/tinyquant-core/src/codec/`
   between Phase 14 (first codebook) and HEAD. For each midpoint,
   run the A1 measurement harness. Find the commit where ρ drops
   below the post-A1 baseline by ≥ 0.05.
2. If the bisect lands on a rotation / codebook commit, audit that
   diff for:
   - incorrect `faer` matrix layout (row-major vs column-major)
   - wrong seed propagation
   - integer overflow in codebook index packing
3. If no single commit is responsible, compare Rust scalar code vs
   the Python reference side-by-side for the same 10-vector input.
   Identify the first divergence.

**Repair:** case-by-case. The repair should be covered by a new
unit test in `tests/reference/` that pins the fixed behavior against
the Python reference at ≥ 1 e-5 per-element absolute difference (or
whatever bit-parity level Phase 16 established).

**Exit criterion:** with B1-measured thresholds reverted to plan
values (or narrowly above them), pr-speed passes.

### B4 — Fixture refresh

**When:** H4 confirmed (A7 shows degenerate corpus).

**Steps:**

1. Regenerate the fixture from the documented upstream source
   (OpenAI `text-embedding-3-small` sample from Phase-21 plan, or
   whatever the provenance doc says).
2. Update `rust/crates/tinyquant-bench/fixtures/calibration/manifest.json`
   with new SHAs.
3. Re-run Phase A to record new thresholds, then pick a B1 / B2 / B3
   path based on the new numbers.
4. Note the regeneration + regenerated-on date in `CHANGELOG.md`.

### B5 — Phase 21 bisect

**When:** H5 confirmed (A6 replay on pre-Phase-21 HEAD shows better ρ).

**Steps:**

1. `git bisect start HEAD <pre-phase-21-sha>` in a dedicated
   worktree.
2. Bisect harness: pr-speed bw4-off run, success if ρ ≥ 0.97.
3. Once the first-bad commit is identified, repair in-place (not
   revert — preserve the rayon batch path's perf improvement).

## 6. Integration (Phase C)

After Phase B closes — regardless of which branch(es) fired:

### C1. Update `risks-and-mitigations.md`

Add or update a row for calibration determinism / thresholds. Use
the existing R-series numbering.

### C2. Document measurement provenance

Add a short section to
`docs/design/rust/phase-21-implementation-notes.md` (or a new
phase-26 notes file if Phase 26 is scoped) that records:

- the date ground truth was measured
- the toolchain + platform used
- the Python reference version + its SHA
- the measurement numbers (ρ, recall, ratio) per (bw, residual)

This is the authoritative record for future `Threshold` edits.

### C3. Gate the workflow

Decide whether `rust-calibration.yml` stays `workflow_dispatch`-only
or whether it should wire into the release path. Options:

- **Soft gate**: run on every tag push before the real publish; fail
  closed. Adds 2–4 h to release cycle.
- **Hard gate**: prerequisite for merging the release chain to main.
  Adds a manual click.
- **No gate**: keep as today (ceremonial only, on-demand).

C3 is a product decision, not a code change. Record the decision in
`docs/design/rust/release-strategy.md`.

### C4. Update the release chain

If B1 (relax) fires, the `CHANGELOG.md` `[Unreleased]` section must
acknowledge the baseline refresh before the release chain merges. If
B2 / B3 fires, the changelog needs a "fixed" entry in the relevant
phase's section.

### C5. Cross-file prose alignment

Per `AGENTS.md` rule: any prose edit to
`tests/calibration.rs`, `README.md`, `COMPATIBILITY.md`, or the
`.github/README.md` "Language bindings" table that mentions
calibration must be mirrored to the other three within the same PR.

## 7. Measurement ledger (fill in during Phase A)

| Metric | bw=2 off | bw=4 off | bw=2 on | bw=4 on | bw=8 on |
|---|---|---|---|---|---|
| ρ (pre-investigation) | 0.5119 | 0.9573 | — | — | — |
| recall@10 (pre-investigation) | unknown | unknown | unknown | unknown | unknown |
| ratio (pre-investigation) | — | — | 1.78 | 1.60 | 1.33 |
| ρ (Python reference)  | 0.4899 | 0.9574 | 1.0000 | 1.0000 | 1.0000 |
| recall@10 (Python reference) | 0.3410 | 0.7920 | 1.0000 | 1.0000 | 1.0000 |
| ratio (Python reference) | 16.0000 | 8.0000 | 1.7778 | 1.6000 | 1.3333 |
| ρ (pre-Phase-21 Rust) | 0.5119 | 0.9573 | 1.0000 | 1.0000 | 1.0000 |
| recall@10 (pre-Phase-21 Rust) | 0.3530 | 0.7910 | 1.0000 | 1.0000 | 1.0000 |
| ratio (pre-Phase-21 Rust) | 16.0000 | 8.0000 | 1.7778 | 1.6000 | 1.3333 |

> [!note] Python oracle agreement
> Python reference and Rust HEAD agree on ρ to within 0.022, recall to
> within 0.012, and ratio bit-exactly. This confirms the Rust codec
> matches its own reference — gaps vs. plan-doc targets are
> design-level (§5 B2), not implementation bugs. A6 pre-Phase-21 Rust
> measurements are bit-identical to HEAD (Phase 21 batch paths are
> numerically equivalent to the serial path).

> [!note]
> Fill this table in during A1 / A2 / A6. This table plus the
> "source of ground truth" in C2 is the long-lived artefact of the
> investigation.

## 8. Acceptance criteria

Phase A "done":
- [x] A1, A2, A3, A4, A5, A7 complete — all cells in §7 filled in
- [x] A6 run (or explicitly deferred) with rationale recorded — ran to
      completion (~40 min), bit-identical to HEAD
- [x] Hypothesis tree in §3 updated with posterior probabilities and
      struck-through hypotheses
- [x] A concrete remediation branch selected from §5 — B1 (relax to
      measured+margin with TODO(phase-26)); B2 deferred per §9

Phase B "done" (per branch):
- [x] All 5 pr_speed tests pass on Windows x86_64 MSVC (verified
      2026-04-14, 508.42s, `--release --features simd`). Linux verified
      previously via Docker bit-identical numbers in §2.1.
- [x] Full suite (5 pr_speed + 5 full_bw* on `openai_10k_d1536`) passes
      on Windows x86_64 MSVC — `cargo test --release -p tinyquant-bench
      --features simd -- --ignored` 10/10 ok in 31953.17s (~8.9 hr),
      verified 2026-04-15 on `develop`. `rust-calibration.yml` workflow
      dispatch still to be exercised for the 2 `core-*` matrix cells.
- [x] No change to existing `cargo test --workspace` green (only
      calibration.rs constants changed; no source-code touched)

Phase C "done":
- [x] risks-and-mitigations.md updated — R22 added
- [x] Measurement provenance doc written — this §7 ledger + Python
      oracle agreement note
- [ ] Workflow gate policy recorded in release-strategy.md — deferred
      to Phase 26 when residual encoder lands (current gates are
      regression-canary, not release quality)
- [x] Changelog has the appropriate entry — `### Calibration` under
      `[Unreleased]`
- [x] Cross-file prose alignment verified — plan doc, risks ledger,
      changelog, and calibration.rs comments all reference the same
      measurements and Phase 26 follow-up

## 9. Scope guardrails

Explicitly out of scope for this investigation plan:

- **Threshold improvements that require algorithm changes**. Those
  belong in a separate Phase 26 plan. This plan is limited to
  diagnosing the current state and either (a) relaxing thresholds
  to match, or (b) pointing at the phase that must be written next.
- **Non-x86_64 calibration**. arm64 is out of scope until x86_64
  clears.
- **Python reference changes**. The Python reference is the gold
  standard; do not modify it to fit Rust numbers. If they disagree,
  Rust is wrong.
- **Fixture-level changes** beyond B4 (regenerate from documented
  source). Do not hand-tune or prune the fixture.
- **CI workflow scope changes** in rust-calibration.yml, beyond the
  `features: ""` → `features: "--features simd"` fix for the
  tinyquant-bench rows (which is almost certainly independently
  desirable, see §11). Any larger workflow restructuring waits.

## 10. Rollback and abort criteria

Abort this investigation and escalate to the user if:

- Phase A3/A4 reveals that the thresholds were hand-written without
  any measurement or plan-doc grounding (nothing to re-baseline
  against).
- Phase A2 shows the Python reference also fails its own thresholds,
  *and* no older commit of either impl has ever passed them
  (indicates a fundamental design gap that needs product-level
  direction).
- Phase B3 bisect lands in `faer` or `rand_chacha` internals
  (upstream dep issue; not a local fix).

Rollback path if a B-branch regresses something else:
- All Phase B changes land on a dedicated branch (`calibration-fix/*`)
  off `phase-21-rayon-batch`. Reverting the branch from main is a
  single `git revert -m 1 <merge>` if the change misfires.

## 11. Incidentally discovered: workflow feature flag gap

During investigation we established that the `rust-calibration.yml`
workflow passes `features: ""` for the four tinyquant-bench rows:

```yaml
- { name: "pr-speed",          filter: "pr_speed",             pkg: "-p tinyquant-bench", features: ""              }
- { name: "full-bw2",          filter: "full_bw2",             pkg: "-p tinyquant-bench", features: ""              }
- { name: "full-bw4",          filter: "full_bw4",             pkg: "-p tinyquant-bench", features: ""              }
- { name: "full-bw8",          filter: "full_bw8",             pkg: "-p tinyquant-bench", features: ""              }
```

…which builds `tinyquant-core` with `default = []` (no `std`, no
`simd`). The test numbers happen to be identical to the `--features
simd` build on x86_64 — so this did NOT cause the observed failures —
but the tinyquant-bench crate's module header says:

> CI gate: `cargo test --workspace --all-features -- --ignored`

Either the workflow should pass `--features simd` to make the SIMD
code path the gated one, or the tinyquant-bench crate should declare
`default = ["simd"]`. Tracked here but may be fixed out-of-band in a
small workflow hygiene commit; not a blocker for the investigation.

## 12. Work log (fill in as the investigation progresses)

| Date | Step | Result | Commit / worktree |
|---|---|---|---|
| 2026-04-14 | pr-speed local run, Windows x86_64 | 5/5 FAIL, see §2.1 | `phase-22-25-release-chain` @ `8e0444d` |
| 2026-04-14 | pr-speed retry with `--features simd`, Windows | 5/5 FAIL, bit-identical numbers | same |
| 2026-04-14 | pr-speed Linux container `rust:1.81` | 5/5 FAIL, bit-identical numbers | same worktree via Docker bind-mount |
| 2026-04-14 | A1 pr-speed metrics extraction | bit-identical numbers captured, logs in `rust/a1-pr-speed.log` | `.worktrees/a1-pr-speed` |
| 2026-04-14 | A2 Python reference cross-check | Python oracle matches Rust within 0.022 ρ / 0.012 recall; ratio bit-exact | `.worktrees/a2-python-reference` |
| 2026-04-14 | A3 threshold provenance dig | Thresholds authored as plan-doc aspirational targets; no measurement record | `.worktrees/a3-threshold-history` |
| 2026-04-14 | A4 plan-doc reconciliation | Plan asserted 4/7/14× ratios and 0.85-0.98 ρ without residual-encoder scope | same worktree |
| 2026-04-14 | A5 ratio ceiling math | Structural cap `4/(bw/8+2)` confirmed — residual=on cannot exceed ~1.78× with raw fp16 residuals | same worktree |
| 2026-04-14 | A6 pre-Phase-21 rebuild | Bit-identical to HEAD after ~40min serial run; H5 (Phase-21 regression) ruled out | `.worktrees/a6-pre-phase-21` |
| 2026-04-14 | A7 hypothesis posterior update | H1+H2 HIGH confirmed; H3, H4, H5 ruled out | this commit |
| 2026-04-14 | B1 selected (thresholds relaxed + TODO(phase-26)) | B2 deferred per §9 to future Phase 26 residual-encoder plan | `calibration-fix/honest-thresholds` |
| 2026-04-14 | B1 complete — 5/5 pr_speed pass locally | `cargo test --release -p tinyquant-bench --features simd pr_speed` 508.42s ok | same branch |
| 2026-04-14 | C1–C5 — risks R22, changelog, plan-doc updates, cross-file alignment | §8 checkboxes in this doc | same branch |

## 13. References

- `rust/crates/tinyquant-bench/tests/calibration.rs` — test file under
  investigation (thresholds at lines 32–56, gate at lines 137–174).
- `rust/crates/tinyquant-core/src/codec/` — codec implementation.
- `rust/crates/tinyquant-core/src/codec/service.rs:212–235` — the
  `std`/`no_std` batch-compress fork that motivates A6.
- `rust/crates/tinyquant-bench/fixtures/calibration/manifest.json` —
  fixture SHAs (verified 2026-04-14).
- `.github/workflows/rust-calibration.yml` — the matrix workflow
  whose first real run surfaced this.
- `tests/reference/tinyquant_py_reference/` — Python reference for A2.
- `docs/design/rust/risks-and-mitigations.md` — target for the R20
  addition in B1 / C1.
- `docs/plans/rust/phase-21-rayon-batch-and-calibration.md` — the
  plan doc that the thresholds claim to derive from. Read as part of
  A4.
- `CLAUDE.md` — declares the Python reference as the behavioral gold
  standard. Drives A2's authority.
