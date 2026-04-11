---
title: "Phase 14: Codebook and Quantize Kernels — Implementation Notes"
tags:
  - design
  - rust
  - phase-14
  - codebook
  - quantize
  - implementation-notes
  - retrospective
date-created: 2026-04-10
date-completed: 2026-04-10
status: stable
category: design
---

# Phase 14 — Implementation Notes

> [!info] Purpose
> Captures the design decisions and deviations from the original
> [[plans/rust/phase-14-codebook-quantize|Phase 14 Plan]] that arose
> during execution on 2026-04-10. Future Rust phases should read this
> alongside [[design/rust/numerical-semantics|Numerical Semantics]]
> §Quantization and §Codebook training for the parity invariants that
> are now locked-in by code and fixtures rather than by prose alone.

> [!note] Relationship to the plan doc
> The plan doc describes the *intended* TDD sequence. This page
> records the *actual* outcome: what landed, what was reshaped
> mid-execution, and why. The plan doc itself has been flipped to
> `status: complete` — treat it as the intent-of-record and this page
> as the execution log.

## What landed

Phase 14 shipped on the dedicated `phase-14-codebook-quantize` branch
and merges back to `main` via an explicit merge commit (per the
user's branch-workflow decision captured in
`C:\Users\aaqui\.claude\plans\tidy-singing-whistle.md`). The branch
contents split into logically-reviewable commits:

1. `chore(workspace)` — add `bytemuck` as a `tinyquant-core`
   dev-dependency via the workspace table, extend `.gitattributes`
   with LFS globs for the two new fixture dirs.
2. `feat(tinyquant-core)` — `Codebook` value object with
   construction invariants, `train`, `quantize_into`, `dequantize_into`,
   and the `PartialEq`-by-bits / custom `Debug` impls.
3. `feat(tinyquant-core)` — private `scalar_quantize` /
   `scalar_dequantize` kernels in `codec::quantize` that the public
   `Codebook` methods delegate to.
4. `feat(xtask)` — `fixtures refresh-codebook` +
   `fixtures refresh-quantize` + extended `refresh-all`.
5. `feat(scripts)` — `generate_rust_fixtures.py codebook` and
   `quantize` subcommands.
6. `test(tinyquant-core)` — freeze the 10 000 × 64 training corpus
   plus the `bw={2, 4, 8}` codebook + quantize fixtures in Git LFS.
7. `docs(rust)` — mark Phase 14 complete; append `log.md`; land this
   implementation-notes page.

New public surface in `tinyquant-core`:

- `tinyquant_core::codec::Codebook` — value object with `new`,
  `train`, `num_entries`, `bit_width`, `entries`, `quantize`,
  `quantize_into`, `dequantize`, `dequantize_into`. Inner storage is
  `Arc<[f32]>` so `Clone` is `O(1)`; equality compares the f32 bits.
- `tinyquant_core::codec::quantize::{scalar_quantize,
  scalar_dequantize}` — `pub(crate)` inside a `pub(crate)` module
  (clippy `redundant_pub_crate` rewrote these to `pub`). These are
  the panic-free reference kernels that the future SIMD path will
  diff against.

Prelude re-exports `Codebook` alongside the Phase 13 surface.

## Deviations from the original plan

### Bit-width sweep instead of a single fixture

The draft plan captured a single training + quantize fixture at
`(seed=42, bit_width=4, n=10000, d=64)`. On review the user requested
the full `bw ∈ {2, 4, 8}` sweep. The training corpus is shared across
all three, so the fixture footprint only grew by 16 + 64 + 1024 = 1104
bytes of codebook entries plus 3 × 10 000 bytes of expected indices —
negligible compared to the 2.5 MiB training corpus. Worth it: we now
have byte parity on 4 + 16 + 256 codebook entries and 30 000 quantize
indices rather than just 16 + 10 000.

### No `proptest`, yes `rand_chacha` loop

The original plan called for a `proptest` proptest to show that
`quantize_into` always produces indices in `[0, 2^bit_width)`. Adding
`proptest = "1"` to the workspace dev-deps pulled a transitive
dependency (`getrandom 0.4.2`, ultimately via modern `tempfile` and
`rustix`) that requires Cargo's `edition2024` feature, which is only
stable from Rust 1.85 onward. The workspace MSRV is 1.81, pinned by
`rust-toolchain.toml` and the `rust-version` field in
`rust/Cargo.toml`, and Phase 12 already bumped from 1.78 once — we
did not want to move it again for a testing convenience.

Instead, the property is expressed as a deterministic ChaCha-seeded
loop (256 batches × up to 512 finite `f32` values drawn from
`ChaCha20Rng::seed_from_u64(1337)`). `rand_chacha` is already a
`tinyquant-core` runtime dependency, so this adds nothing to the
dependency graph. The test covers the same invariant as the original
proptest would have and is trivially reproducible.

### Runtime `fs::read` fixture loading, not `include_bytes!`

The draft plan showed the training-parity test using
`include_bytes!`. That works for small constants, but the Phase 14
training corpus is 2.5 MiB and would have bloated every integration
test binary that linked to `tinyquant-core`. Instead, both new
integration test files follow the exact pattern already in use for
`rotation_fixture_parity.rs`: compute the path with
`env!("CARGO_MANIFEST_DIR")` and read at runtime via `std::fs::read`.
Fixture data therefore only materializes when a test asking for it
runs.

### Tie-break confirmation test had the wrong expected literals

While writing the `quantize_then_dequantize_returns_nearest_entries`
round-trip test I pre-computed expected entries using
`(0..16).map(|i| i as f32 * 0.1)`, which is *not* the same as
`{0.0, 0.1, 0.2, …, 1.5}` — `9.0_f32 * 0.1_f32` is
`0.90000004`, not `0.9`. The first run surfaced the mismatch cleanly
("`[…, 0.90000004, …]` vs `[…, 0.9, …]`"), so the test now compares
`out[k]` to `entries[expected_idx]` directly rather than to a
hand-typed literal. No change to the production code was needed —
this was a test-author gotcha, noted here so future tests in the
codec module use the same pattern.

### Lint collisions and their fixes

The crate denies `clippy::pedantic + nursery + unwrap_used +
expect_used + panic + indexing_slicing + cognitive_complexity`, which
caught several first-draft mistakes before they landed:

- `bool_to_int_using_if` — rewrote
  `if entries.is_empty() { 0 } else { 1 }` as
  `u32::from(!entries.is_empty())`.
- `explicit_iter_method` — replaced `for &v in entries.iter()` with
  `for &v in &*entries` (deref coercion on `Box<[f32]>`).
- `cast_precision_loss` + `cast_possible_truncation` +
  `cast_sign_loss` — the quantile arithmetic legitimately needs
  `k as f64`, `last_idx as f64`, `h.floor() as usize`, and
  `value_f64 as f32`. Rather than fighting the casts, `Codebook::train`
  carries a local `#[allow(...)]` attribute naming exactly those three
  lints. No other function needs the allow.
- `missing_fields_in_debug` — the hand-written `Debug` impl now
  includes the `entries` slice, not just `bit_width` and
  `num_entries`.
- `redundant_pub_crate` — because `codec::quantize` is already
  `pub(crate)`, inner items must be `pub` rather than `pub(crate)`.
- `trivially_copy_pass_by_ref` — the `f32_cmp` helper takes
  `f32, f32` by value.

All of these are documented here so the next phase doesn't reinvent
the workaround.

### No changes to the Python reference

Phase 14 deliberately does not touch `src/tinyquant_cpu/codec/`. The
Python codebook is the parity source of truth; the Rust port mirrors
it. If Python-side drift ever surfaces through the fixture diff, the
right move is to investigate the Python change first, not to relax
the Rust comparison.

## Test evidence

After the final green run:

- **12 codebook tests** — four construction invariants, bw=2/4/8
  training parity, two quantize/dequantize round-trips, bw=2+bw=8
  bounds smoke, dequantize out-of-range rejection, and the
  ChaCha-seeded in-range scan.
- **4 quantize tests** — bw=2, bw=4, bw=8 Python-fixture byte parity
  (10 000 indices each, 30 000 total) and a bw=4 dequantize round
  trip that every output f32 belongs to the codebook entry set.
- **No regressions** — all Phase 12 and Phase 13 tests continue to
  pass (`codec_config` 13, `rotation_matrix` 8,
  `rotation_fixture_parity` 3, `rotation_cache` 8, plus the pre-
  existing workspace baseline).

`cargo xtask fmt`, `cargo xtask lint`, `cargo xtask test`,
`cargo build -p tinyquant-core --no-default-features`, and
`cargo build -p tinyquant-core --target thumbv7em-none-eabihf
--no-default-features` were all green on the final sweep. Running
`cargo xtask fixtures refresh-all` after the code landed left
`git status` empty against the committed fixture blobs, proving the
Python + Rust fixture round trip is deterministic.

## Lessons learned (surfaced during the PR cycle)

These observations came *after* the initial draft of this page,
during the PR review cycle against `rust-ci.yml`. Every one of them
is a process lesson rather than a numerical lesson — the code
itself was correct the first time — but they changed how much trust
we should place in a "green locally" claim going forward.

### L1. Pre-merge CI is the quality gate, not local tests

Phase 13 landed bit-exact rotation-fixture parity tests and the
implementation notes confidently asserted "Fixture reproducibility
confirmed by `md5sum` before and after a fresh
`cargo xtask fixtures refresh-all`" — but only on the author's
Windows dev machine. The tests had **never** successfully run on
CI. When the Phase 14 PR triggered the first honest rust-ci
lookback, the full history was `0 / 3 successful runs`: every push
to `main` touching `rust/**` since Phase 11 had been silently red.

**What to do differently.** After pushing a branch, always confirm
at least one successful run of the workflow that owns the new code,
even when local tests are green. `gh run list --workflow <name>
--branch main --limit 5` is the single command that would have
flagged this in Phase 13. Treat any `completed failure` on `main`
as a blocker for landing the next phase, not background noise.

### L2. LFS hydration on `actions/checkout@v4` is off by default

Phase 13's rotation fixtures were 32 KiB / 4.7 MiB of raw `f64`
bytes tracked by Git LFS. On CI, `actions/checkout@v4` pulls the
132-byte LFS pointer files unless `with: { lfs: true }` is set.
The downstream test then reads a pointer, sees `132 != 32768`, and
panics with a length assertion. Because the Phase 13 tests were
never watched on CI, nobody noticed. Phase 14's codebook training
corpus (2.5 MiB) hit the same bug with a noisier signature.

**What to do differently.** Whenever a workflow tests code that
reads LFS-tracked content, the checkout step **must** carry
`with: { lfs: true }`. If the design doc already claims this
(`docs/design/rust/ci-cd.md` does, at the time of writing), verify
the YAML. The fix for rust-ci landed in commit `13e888d` as part
of this PR.

### L3. Design docs are not CI config

`docs/design/rust/ci-cd.md` at line 239 says "Fixture files are in
Git LFS; `actions/checkout` with `lfs: true` on every job." The
actual `rust-ci.yml` did not follow this. The design intent was
written down before the workflow was implemented, and no automated
check verified that the YAML stayed in sync. This is a pattern we
should assume holds for other design docs too.

**What to do differently.** When a design doc makes a claim about
tooling behavior ("every job has X", "every PR runs Y"), add a
test or grep that would fail if the claim drifts from reality. At
minimum, spot-check the claim against the YAML during each phase
execution. A future sweep should grep `docs/design/rust/ci-cd.md`
for testable claims and diff them against `.github/workflows/`.

### L4. Cross-platform bit-exact parity requires determinism
contracts *in code*, not CI workarounds

The Phase 13 rotation pipeline uses `faer::Mat::qr()`, which calls
faer's default parallel Householder reduction. At `dim=768`, Linux
and Windows CI runners pick different parallel reduction orders,
producing floating-point outputs that differ by ~90% of the f64
words — not a 1-ULP edge case, a structural divergence. The
`dim=64` matrix still matched bit-exact because it falls below
faer's parallel kernel threshold.

Phase 14 worked around this by pinning `RAYON_NUM_THREADS: "1"` on
the rust-ci `Test` job (commit `40f9b87`). That keeps the committed
fixture authoritative without regenerating it on Linux, but it is
explicitly a workaround — a determinism contract belongs inside
`RotationMatrix::build` via an explicit `faer::Parallelism::None`
(or equivalent serial code path), not in the workflow env vars.

**What to do differently.** Any future "Rust-canonical" fixture
that is supposed to be bit-exact across platforms **must** be
accompanied by a code-level determinism contract: explicit
`Parallelism::None`, documented rayon-pool sizing, and a test that
verifies the fixture matches under varying thread counts. The
long-term fix for the current rotation builder is in the Phase
14 § CI follow-ups below.

### L5. `f32` literal tie-breaks in round-trip tests are a
foot-gun

While writing the codebook round-trip test
(`quantize_then_dequantize_returns_nearest_entries`) I pre-computed
expected entries as `(0..16).map(|i| i as f32 * 0.1)` and then
compared to `0.9f32`. That comparison fails at index 9 because
`9.0_f32 * 0.1_f32` rounds to `0.90000004`, not `0.9`. The first
test run surfaced the mismatch cleanly
(`[…, 0.90000004, …]` vs `[…, 0.9, …]`), and the fix was to
compare to `entries[expected_idx]` rather than a hand-typed
literal. No production code changed.

**What to do differently.** When a test needs to assert that a
dequantized value matches a specific codebook entry, always read
the entry back out of the codebook under test rather than
rewriting the arithmetic in the test source. Codec tests in Phase
15+ should default to `assert_eq!(out[i], cb.entries()[j])` over
`assert_eq!(out[i], 0.9)`.

### L6. Test-framework MSRV creep is real

`proptest = "1"` looked like a harmless dev-dependency add until
its transitive dep tree pulled `getrandom 0.4.2` (via
`tempfile → rustix`), which requires Cargo's `edition2024` feature
stable only in Rust 1.85. The workspace MSRV is 1.81 and Phase 12
already bumped from 1.78 once. We declined to bump again for a
test-framework convenience and substituted a deterministic
`rand_chacha::ChaCha20Rng::seed_from_u64(1337)` loop that shares
the same runtime dep (no new crates in the graph).

**What to do differently.** Before adding any dev-dependency, run
`cargo tree -p <crate> --prefix depth --depth 3` to see what it
brings in, and cross-check against `rust-toolchain.toml`. Prefer
crates that declare a permissive MSRV or that can be pinned to an
older minor if the graph bites. For property-style invariants, a
deterministic seeded loop over an existing RNG crate is a valid
substitute that sidesteps the issue entirely.

### L7. Clippy profile gotchas worth remembering

The crate denies `clippy::pedantic + nursery + unwrap_used +
expect_used + panic + indexing_slicing + cognitive_complexity`,
which caught five first-draft mistakes in the Phase 14 code:

- `bool_to_int_using_if` — use `u32::from(!slice.is_empty())`, not
  `if slice.is_empty() { 0 } else { 1 }`.
- `explicit_iter_method` on `Box<[f32]>` — use `for &v in &*boxed`,
  not `for &v in boxed.iter()`.
- `missing_fields_in_debug` — hand-written `Debug` impls must cover
  every field or use `.finish_non_exhaustive()`.
- `redundant_pub_crate` — inside a `pub(crate)` module, inner items
  must be `pub`, not `pub(crate)`.
- `trivially_copy_pass_by_ref` — `f32`/`f64` helpers take values,
  not references.

Narrow-scope `#[allow(clippy::cast_precision_loss,
clippy::cast_possible_truncation, clippy::cast_sign_loss)]` is
preferred over loosening the crate-wide profile, and should sit on
the single function that needs it (in Phase 14, only
`Codebook::train` has the attribute).

**What to do differently.** Phase 15+ code should re-read this
list before the first clippy run — these are recurring patterns,
not one-offs.

## CI follow-ups queued after Phase 14

These items landed on main as part of the Phase 14 PR but are
**Phase 13 debt**, not Phase 14 scope. They should be cleared in a
dedicated remediation PR before Phase 15 starts building on top of
them:

1. **Determinism contract in `RotationMatrix::build`.** Replace
   `a.qr()` in
   `rust/crates/tinyquant-core/src/codec/rotation_matrix.rs:78`
   with an explicit `faer::Parallelism::None` (or equivalent serial
   path) so the cross-platform bit-exact guarantee lives in code
   rather than in the rust-ci workflow env. Once that lands, drop
   the `RAYON_NUM_THREADS: "1"` override from
   `.github/workflows/rust-ci.yml`.
2. **Toolchain drift in rust-ci.yml.** All four jobs still declare
   `toolchain: "1.78.0"` on the `dtolnay/rust-toolchain` action,
   even though the workspace MSRV has been 1.81 since Phase 12.
   This works because `rust-toolchain.toml` auto-installs 1.81 at
   cargo-invocation time, but it wastes an install per job. Bump
   the four occurrences to `"1.81.0"` and add a lint verifying the
   two values stay in sync.
3. **CI health check in the phase playbook.** Add a Phase-end
   checklist item: after pushing a PR that touches `rust/**`, run
   `gh run list --workflow rust-ci.yml --branch main --limit 5`
   and confirm no `completed failure` entries remain on `main`.
   This would have flagged the Phase 13 LFS issue months earlier.

## What Phase 15 inherits

Phase 15 (Codec service + residual) can assume:

- `Codebook::train` produces byte-identical entries for the same
  `(seed, rows, cols, bit_width)` triple Python does — the test
  suite freezes the `seed=42, rows=10000, cols=64, bw ∈ {2, 4, 8}`
  snapshot.
- `Codebook::quantize_into` is byte-for-byte compatible with
  `tinyquant_cpu.codec.codebook.Codebook.quantize` on f32 inputs
  drawn from `numpy.random.default_rng(7).standard_normal`.
- `scalar_quantize` and `scalar_dequantize` are stable reference
  kernels: Phase 20's SIMD path can diff against them crate-
  internally without going through `Codebook`.
- `Codebook` is `no_std`-safe, `Clone + Debug + PartialEq`, and
  panic-free under the codec crate's full lint profile.

## See also

- [[plans/rust/phase-14-codebook-quantize|Phase 14 Plan]]
- [[design/rust/numerical-semantics|Numerical Semantics]]
- [[design/rust/phase-13-implementation-notes|Phase 13 Implementation
  Notes]]
- [[plans/rust/phase-15-codec-service-residual|Phase 15 Plan]]
