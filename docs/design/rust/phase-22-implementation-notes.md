---
title: "Phase 22 Implementation Notes"
tags:
  - design
  - rust
  - phase-22
  - pyo3
  - cabi
  - release
date-created: 2026-04-14
category: design
---

# Phase 22 Implementation Notes

## Summary

Phase 22 splits into four parts (A/B/C/D). At the time of writing:

- **Part A — `tinyquant-py` PyO3 wheel (LANDED).** Commits `978ebf1`
  (initial wheel) and `158a132` (code-review fixes) ship the
  `tinyquant_cpu` parity surface: `CodecConfig`, `Codebook`,
  `CompressedVector`, `Codec.compress` / `.decompress` /
  `.compress_batch`, the frozen exception hierarchy
  (`TinyQuantError` → `DimensionMismatchError`, `ConfigMismatchError`,
  `CodebookIncompatibleError`, `DuplicateVectorError`), Python
  `__reduce__` round-trip, byte-equality parity tests against the
  Python reference, and GIL-release paths on every expensive call.
  cibuildwheel targets are wired into CI.
- **Part B — `tinyquant-sys` C ABI (LANDED at `f1eae7c`, fixed on
  this branch).** Opaque handle types (`CodecConfigHandle`,
  `CodebookHandle`, `CompressedVectorHandle`, `CorpusHandle`,
  `ByteBufferHandle`), `TinyQuantErrorKind` / `TinyQuantError`
  out-pointer, `catch_unwind`-wrapped `extern "C" fn` entry points for
  single-vector compress / decompress, compressed-vector
  (de)serialisation, and corpus CRUD. `build.rs` regenerates
  `include/tinyquant.h` via cbindgen on every crate build; CI gates
  the committed header with `git diff --exit-code`. Test surface:
  `abi_smoke`, `abi_handle_lifetime`, `abi_header_compile`,
  `abi_c_smoke`, `abi_cxx_smoke`, `abi_panic_crossing` (behind
  `panic-probe` feature).
- **Part C — `tinyquant-cli` standalone binary (PENDING).**
- **Part D — release workflow + tag-driven publish (PENDING).**

This note documents Part B + the follow-up fixes from the spec
reviewer's ISSUES FOUND round. Part A is already covered by its own
commit messages and the Phase 22 plan doc.

## Deviations from spec

### 1. Error-kind discriminants `Panic = 98` and `Unknown = 99`

`docs/plans/rust/phase-22-pyo3-cabi-release.md` and the companion
design note `docs/design/rust/ffi-and-bindings.md` §Binding 2 gave
sentinel discriminant suggestions in the 250-range (e.g. `Panic =
254`, `Unknown = 255`). The implementation ships `Panic = 98` and
`Unknown = 99` instead.

Rationale, preserved from the Phase 22.B implementer's notes:

- **Readability in C consumer code.** `TQ_ERR_PANIC == 98` is easier
  to spot in log output and gdb watchpoints than `254`. Two-digit
  values do not look like "bit flags gone wrong".
- **Small-integer discipline.** The `TinyQuantErrorKind` enum is
  `#[repr(C)]` backed by the platform's default C-enum width. A
  8-bit `uint8_t` is not guaranteed, so using values near `u8::MAX`
  buys nothing portability-wise, but it does look like a sentinel
  that callers may want to bit-test — which is not the API contract.
- **No collision risk.** The 0.x line adds variants at the low end
  of the range (next slot is `Io = 6` → `InvalidArgument = 7` →
  future additions go into `8`, `9`, `10`, …). Leaving a gap from
  `7` to `98` reserves ~90 slots for additional well-classified
  errors before ever approaching the `Panic` / `Unknown`
  "wrapper-internal" discriminants.

The header-level contract is frozen for the 0.x line either way —
changing a discriminant is a breaking change, so the specific numbers
are not a blocker.

### 2. §Binding 2 batch + threadpool functions deferred to Phase 22.C

§Binding 2 of `docs/design/rust/ffi-and-bindings.md` lists three
functions that are NOT present in the landed 22.B crate:

- `tq_set_num_threads(n: u32) -> TinyQuantErrorKind` — configure the
  Rayon pool used by batch paths.
- `tq_codec_compress_batch(... *mut *mut CompressedVectorHandle, ...)`
  — parallel batch compress.
- `tq_codec_decompress_batch(... *const *const CompressedVectorHandle,
   f32 *out, ...)` — parallel batch decompress.

**Decision: defer to Phase 22.C follow-up commit on this same
branch.** Rationale:

- `tq_codec_compress_batch` must return an *array* of
  `CompressedVectorHandle*` to the caller. That introduces a new
  ownership rule ("caller owns every element; use `tq_bytes_free`
  plus `tq_compressed_vector_free` in a loop to tear down") which
  warrants a dedicated `abi_handle_lifetime` test-suite expansion
  and an `AGENTS.md`-style ownership rule in `tinyquant.h`. That is
  more than the "small rayon wrap" the blocking-issues note
  suggested.
- `tq_set_num_threads` needs to decide between `rayon::ThreadPool`
  (per-library pool installed via `pool.install(||
  compress_batch_with(...))`) and the Rayon global pool. Phase 21
  adopted per-call `pool.install`, and Part A (`tinyquant-py`)
  inherits the global-pool default. The C ABI should match *one* of
  those; picking correctly is a design call that belongs with the
  CLI crate (Part C), which also needs the same thread-pool story.
- Parts A and B are a release-ready subset without the batch C
  functions: every downstream consumer that currently exists (Python
  wheel, the CLI that Part C will add) goes through the native Rust
  crate, not the C ABI. The C ABI is for *external* consumers and
  they are content to loop over single-vector `tq_codec_compress`
  until 22.C lands the batch entry points.

**Concrete plan for Phase 22.C:**

- Add `tq_set_num_threads` that builds a `rayon::ThreadPoolBuilder`
  and stores the pool in a `OnceLock<Arc<ThreadPool>>` inside
  `tinyquant-sys::internal`.
- Add `tq_codec_compress_batch` + `tq_codec_decompress_batch` that
  install the stored pool before calling `codec.compress_batch_with`
  / `codec.decompress_batch_into` with `Parallelism::Custom(rayon_driver)`.
- Expand `abi_handle_lifetime.rs` with double-array ownership tests
  and a null-element test for the output buffer.
- Regenerate `include/tinyquant.h` via `cargo build -p tinyquant-sys
  --release` (see §Header generation workflow below) in the same
  commit.

### 3. `abi_handle_lifetime.rs` does not cover double-free / UAF

`rust/crates/tinyquant-sys/tests/abi_handle_lifetime.rs:16-18`
documents the opt-out:

```text
We do NOT test double-free: the contract is "must be called exactly
once per successful *_new", and verifying it would require
instrumenting the heap.
```

**Why:** a double-`Box::from_raw` on the same pointer is undefined
behaviour in safe Rust — there is no deterministic way to assert
"this crashed safely" without either:

- running under **Miri** (which we already do for the scalar kernel
  parity tests in Phase 20, but Miri does not currently model
  `catch_unwind` payloads reliably enough to round-trip a panic
  message), or
- wiring in a **debug-build sentinel** pattern (stamp a magic value
  on `Box::into_raw`, zero it on `Box::from_raw`, check for the
  magic on every dereference) — this is the standard C pattern for
  catching UAF, but it requires every accessor (`tq_*_bit_width`,
  `tq_*_dimension`, …) to branch on the sentinel in **release
  builds too**, or the check is useless. That costs one predictable
  branch per accessor call on the hot path and reshapes the ABI
  (we cannot then return a sentinel on null without ambiguity).

**Decision:** keep the opt-out, document it in the header prose
("must be called exactly once per successful `_new`"), and rely on
the single-pass lifetime tests we do have. A future Phase (22.C or
later) can add a **debug-feature-gated** magic-sentinel path —
e.g. `#[cfg(feature = "debug-handles")]` — without touching the
release ABI. That is a strict addition, not a deviation.

### 4. `abi_panic_crossing.rs` covers only `tq_test_panic`

The current panic-crossing integration test exercises the
`catch_unwind` wrapper via a dedicated probe (`tq_test_panic`),
gated on `feature = "panic-probe"`. It does NOT deliberately force
a panic through every `extern "C" fn` entry point and assert the
result is `TinyQuantErrorKind::Panic`.

**Why:** mechanically forcing a panic through `tq_codec_compress`
or `tq_corpus_insert` would require either:

- injecting a panic hook into `tinyquant-core` internals (invasive,
  couples the core crate to the sys crate's test shape), or
- patching out the codec with a mock that panics on call (needs
  dynamic dispatch in `tinyquant-core::codec::Codec`, which the
  phase-design doc explicitly rejects on performance grounds —
  `Codec::new()` is a zero-size struct).

**Decision:** document as a known gap. The invariant we care about
("`catch_unwind(AssertUnwindSafe(...))` wraps every entry point,
converts panic payloads to `TinyQuantErrorKind::Panic`, and returns
before unwinding into C") is:

- *structurally* enforced by the shape of every `extern "C" fn` in
  `codec_abi.rs` and `corpus_abi.rs` (they all follow the same
  3-line boilerplate and a clippy lint could later grep for the
  pattern), and
- *dynamically* enforced by `tq_test_panic`, which shares the exact
  same `catch_unwind` wrapper.

If a future refactor splits or renames the wrapper, the
`tq_test_panic` probe will catch it at test time, and a follow-up
inspection of the other entry points is cheap. Expanding the test
matrix to every entry point is ~24 additional tests for the same
binary-level guarantee.

### 5. cbindgen `with_src` instead of `with_crate`

`build.rs` feeds cbindgen only `src/lib.rs` via `with_src`, not the
full crate via `with_crate`. Reason carried over verbatim from the
22.B commit message:

> `cbindgen`'s `with_crate` runs `cargo metadata`, which in this
> workspace fails because the pgvector crate transitively references
> a registry package that requires a newer Cargo than our MSRV 1.81.
> `with_src` side-steps metadata by enumerating the Rust sources
> directly — cbindgen parses each file, follows `use crate::...` at
> syntactic level, and never invokes cargo.

**Side-effect addressed in this fix round:** cbindgen's `@version@`
substitution in the `header` config entry is ONLY applied by the
`with_crate` code-path (cargo metadata supplies the version). Under
`with_src`, the `@version@` token passes through literally. The
fixed build script now post-processes the emitted header and
substitutes `@version@` with `env!("CARGO_PKG_VERSION")` at build
time; a `const _: ()` compile-time assertion in `src/lib.rs` catches
drift between the manually-kept-in-sync `TINYQUANT_H_VERSION`
constant and the build-time crate version. See §Header generation
workflow for the command.

### 6. cbindgen does NOT fail the build on generation error

`build.rs` catches a cbindgen `Err` and emits
`cargo:warning=cbindgen failed ...` rather than panicking. That is
deliberate and carried over from the 22.B commit: `cargo publish`
repacks the crate without a full source tree in some environments,
and a hard-failing build script would break crates.io publication.
The CI `abi-header-drift` job still catches any real regression via
`git diff --exit-code rust/crates/tinyquant-sys/include/tinyquant.h`.

## Header generation workflow

The generated header `rust/crates/tinyquant-sys/include/tinyquant.h`
is a **committed source file**, not a build artefact. Regenerate it
with:

```bash
cargo build -p tinyquant-sys --release
```

That invokes `build.rs`, which:

1. Loads `cbindgen.toml`.
2. Runs cbindgen with `with_src(src/lib.rs)` to avoid cargo
   metadata (MSRV 1.81 compatibility).
3. Writes the emitted header to `include/tinyquant.h`.
4. Reads it back and substitutes every `@version@` token with the
   build-time value of `CARGO_PKG_VERSION` (the token in the
   `header = ...` config block that cbindgen does not expand under
   `with_src`).
5. Emits `cargo:rerun-if-env-changed=CARGO_PKG_VERSION` so a crate
   version bump triggers regeneration even when no source under
   `src/` was touched.

### Drift guards

- **Rust compile-time:** `src/lib.rs` declares `pub const
  TINYQUANT_H_VERSION: &str = "0.1.0"` and an adjacent `const _: ()
  = { ... }` block that byte-compares it against
  `env!("CARGO_PKG_VERSION")`. A bump of the workspace
  `[workspace.package] version` without also updating this constant
  is a compile-time error with a clear message.
- **Integration test:** `tests/abi_header_compile.rs` contains a
  dedicated `header_version_macro_matches_crate_version` test that
  reads the committed header file and asserts both the substituted
  value and the absence of any literal `@version@`. This test does
  NOT depend on `libclang`, so it runs on every CI host even when
  the bindgen round-trip inside the same file skips.
- **CI:** `rust-ci/abi-header-drift` runs
  `cargo build -p tinyquant-sys --release` followed by
  `git diff --exit-code rust/crates/tinyquant-sys/include/tinyquant.h`.
  A non-zero diff means the committed header is stale vs. Rust source.
- **`abi_header_compile.rs` early-return on missing libclang.** The
  bindgen round-trip check inside that file is wrapped in
  `std::panic::catch_unwind` and returns a skip message if libclang
  is not discoverable (`libclang` is an indirect dep of `bindgen`).
  That early-return is **load-bearing for minimal CI runners** and
  must not be removed — the version-macro check (above) runs
  unconditionally so we do not lose coverage on libclang-less hosts.

## Open items for Phase 22.C / 22.D

Tracked as work for the remaining Phase 22 parts. None of these
block Parts A + B, but 22.C must clear the first three before
Part D publishes the first tagged release.

- [ ] **22.C.1:** Implement `tq_set_num_threads`,
      `tq_codec_compress_batch`, `tq_codec_decompress_batch` per the
      plan in §Deviation 2 above. Regenerate `include/tinyquant.h`
      in the same commit.
- [ ] **22.C.2:** Expand `abi_handle_lifetime.rs` with array-of-
      `CompressedVectorHandle*` ownership tests (null elements,
      partial-failure teardown, double-free of batch outputs).
- [x] **22.C.3:** `tinyquant-cli` standalone binary per
      `docs/plans/rust/phase-22-pyo3-cabi-release.md` Part C. Landed
      in the §Part C section above.
- [ ] **22.D.1:** Release workflow + tag-driven publish (PyPI +
      crates.io + GitHub Release artefacts) per Part D.
- [ ] **22.D.2:** Consider a `#[cfg(feature = "debug-handles")]`
      sentinel pattern for catching double-free / UAF in debug
      builds (§Deviation 3). Strict addition; does not touch the
      release ABI.
- [ ] **22.D.3:** Evaluate whether to expand `abi_panic_crossing`
      to loop over every `extern "C" fn` (§Deviation 4). Decision
      gate: cost (~24 tests, 1-second runtime) vs. benefit (catch
      a refactor that removes `catch_unwind` from a single entry
      point). Defer unless such a refactor is actually proposed.

## Part C: `tinyquant-cli` standalone binary

Phase 22.C ships a single `tinyquant` binary that exercises the codec
and corpus surfaces over a deterministic, codec-file format. The
crate lives at `rust/crates/tinyquant-cli/`; the binary entry point
is `src/main.rs`, command bodies live under `src/commands/`, and
format-aware matrix I/O lives in `src/io.rs`.

### Subcommand tree

| Command | Purpose |
|---|---|
| `tinyquant info` | Print version, git commit, rustc, target triple, profile, enabled features, runtime ISA, CPU count. |
| `tinyquant codec train` | Train a codebook from raw FP32 (writes `TQCB` + optional JSON sidecar). |
| `tinyquant codec compress` | Compress FP32 matrix into a `TQCV` corpus via `Parallelism::Custom(rayon)`. |
| `tinyquant codec decompress` | Stream a `TQCV` corpus back to FP32. |
| `tinyquant corpus ingest --policy compress` | Thin wrapper over `codec compress` (renames flags to corpus-flavoured names). |
| `tinyquant corpus decompress` | Thin wrapper over `codec decompress` (renames flags). |
| `tinyquant corpus search` | Brute-force top-k search over a `TQCV` corpus with FP32 query. |
| `tinyquant verify <PATH>` | Magic-byte dispatch verifier (`TQCB` / `TQCV` / `TQCX`). |
| `--generate-completion <SHELL>` | Emit `clap_complete` shell completions to stdout. |
| `--generate-man` | Emit a `clap_mangen` man page to stdout. |

### Sidecar formats

The CLI introduces two CLI-internal file formats. Both are pinned to
this binary (the on-wire codec layer continues to use the Level-1 /
Level-2 protocols in `tinyquant-io`):

- **TQCB codebook** (`commands::codebook_io`): 4-byte magic + 1-byte
  version + 1-byte bit width + 2 reserved bytes + `u32` LE
  `num_entries` + `num_entries * f32 LE` entries.
- **JSON config sidecar**: `{ bit_width, seed, dimension,
  residual_enabled, config_hash? }` — produced by `codec train
  --config-out`, consumed by `codec compress` / `codec decompress`
  / `corpus search` / `verify`.

### Exit code taxonomy

`commands::CliErrorKind` is attached via `anyhow::Context` and
unwrapped in `main::report` to pick the process exit code.

| Code | Variant | Meaning |
|---|---|---|
| `0` | — | Success. |
| `2` | `InvalidArgs` | Bad flags, missing sidecars, schema mismatches. |
| `3` | `Io` | Open / read / write failures. |
| `4` | `Verify` | Magic mismatch, header corruption, truncated record. |
| `70`| `Other` | Catch-all (codec internal errors, etc.). |

clap's own `ExitCode 2` for parse errors slots into the same `2`
bucket without any special wiring.

### Deviations from the §Step 11–17 skeleton

1. **Feature-graph narrowing.** The plan implies
   `default = ["jemalloc", "rayon", "simd", "mmap", "progress"]` and
   that each feature toggles the same-named feature on the workspace
   crates. In practice `tinyquant-core` and `tinyquant-bruteforce`
   do not expose a `rayon` feature, and `tinyquant-io::parallelism`
   contains a `clippy::redundant_closure` violation that breaks the
   workspace `-D warnings` gate when its `rayon` feature is on. The
   CLI uses its own per-crate `Parallelism::Custom(fn_pointer)`
   driver inside `pool.install(...)`, so we don't need
   `tinyquant-io/rayon` at all. The CLI's own `rayon` feature is now
   a stub (`rayon = []`) — kept on the surface so the feature flag
   matrix in §22.D doesn't have to change.
2. **MSRV pin storm.** A wave of late-2025 transitive crate releases
   moved to `edition2024`, which requires rustc ≥ 1.85. The repo
   pins MSRV to 1.81, so the CLI explicitly pins:
   - `clap = "=4.5.21"`, `clap_complete = "=4.5.40"`,
     `clap_mangen = "=0.2.24"`,
   - `comfy-table = "=7.1.4"`,
   - `assert_cmd = "=2.0.17"`, `predicates = "=3.1.4"`,
     `tempfile = "=3.14.0"`,
   - and downgrades `pest{,_derive,_meta,_generator}` to `2.7.15`
     and `unicode-segmentation` to `1.12.0` in `Cargo.lock` (these
     are pulled by `npyz → py_literal → pest`).
3. **`codec train --config-out` flag and `CodecCmd::Train { format }`.**
   The §Step 12 skeleton lists the train / compress / decompress chain
   but does not specify how the downstream commands reconstruct
   `CodecConfig` (it's not recoverable from the codebook alone — `seed`
   and `residual_enabled` are config-level). We add `--config-out` on
   `codec train` and matching `--config-json` on every consumer. We
   also surface `--format` on `codec train` so that operators can feed
   the command `.npy` / `.csv` / `.jsonl` training corpora without a
   pre-processing step; this is spec-compliant under §CLI I/O format
   specification, which pins `--format` to every I/O-touching
   subcommand. The §Step 13 Rust code skeleton is illustrative and
   does not enumerate every flag; the §CLI I/O format specification
   table is the authoritative surface.
4. **`corpus ingest --policy {passthrough, fp16}`.** The current
   `tinyquant-io::codec_file` writer only emits compressed records,
   so the passthrough and FP16 policies cannot be implemented
   without first extending the writer. The CLI surface accepts both
   policy values, but immediately returns
   `CliErrorKind::InvalidArgs` with a "not yet supported" error
   pointing operators at `codec compress`. Phase 23+ can fold the
   missing policies in without touching the CLI contract.
5. **Worker-thread stack bump on Windows.** Windows MSVC ships with
   a 1 MiB main-thread stack, which is not enough to run the
   `faer::Mat::qr` decomposition path inside
   `RotationMatrix::build` under a `--profile dev` build. `main()`
   parses args on the small main stack and then hands the entire
   dispatch off to a worker thread with an 8 MiB stack. On Linux
   and macOS the default thread stack is already 8 MiB, so this is
   a no-op there.
6. **`build.rs` emits `TINYQUANT_`-prefixed env vars** instead of the
   bare `GIT_COMMIT`, `RUSTC_VERSION`, `TARGET`, `PROFILE` names that
   the §Step 15 Rust code skeleton shows (`option_env!("GIT_COMMIT")`).
   `src/commands/info.rs` reads the same prefixed names
   (`option_env!("TINYQUANT_GIT_COMMIT")` etc.). Rationale: the bare
   names collide with CI-injected environment variables during Cargo
   builds — GitHub Actions and many self-hosted runners set
   `GIT_COMMIT` / `TARGET` globally, which would make `build.rs` bake
   the wrong values into the binary (the CI job's commit, not the
   crate's; the wrapper's target, not Cargo's). The prefix is a
   defensive decision: it scopes the env-var contract to this crate
   and keeps the `info` output honest on shared runners. `build.rs`
   also emits `cargo:rerun-if-env-changed=TINYQUANT_GIT_COMMIT` so
   an operator override of the commit string rebuilds the binary.
   The §Step 15 skeleton is illustrative code; the header prose that
   describes "git commit / rustc / target / profile" is the authoritative
   contract, and the output format matches it verbatim.
7. **`indicatif` progress bars wired into the batch paths in a follow-up
   commit, not the initial scaffold.** §Step 14 mandates
   `indicatif::ProgressBar` on long-running batches. The initial
   Phase 22.C landing shipped the `progress` Cargo feature and the
   `indicatif = "0.17"` optional dependency but did not instantiate
   a bar in any subcommand body (the feature was a no-op stub). The
   follow-up commit adds `crate::progress`, a global `--no-progress`
   flag on `Cli`, and determinate bars in `codec_compress::run`
   (rayon-driven `compressing` bar ticking per row; serial `writing`
   bar ticking per record), `codec_decompress::run` (`decompressing`,
   per-record), and `corpus_search::run` (`loading corpus`,
   per-record in the pre-query decompress sweep). The `rayon_driver`
   `fn` pointer cannot close over per-call state, so the compress
   bar is published through a `static OnceLock<Mutex<Option<ProgressBar>>>`
   in `crate::progress` (`set_active_compress_bar` before
   `pool.install`, `clear_active_compress_bar` after). The module
   honours `--no-progress`, `TERM=dumb`, and, via `indicatif`'s own
   detection, `NO_COLOR=1`. Corpus ingest delegates to
   `codec_compress::run` and inherits the bar automatically.

### Acceptance signals (Windows host)

| Gate | Result |
|---|---|
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean |
| `cargo build --workspace --release` | clean (compile time ~2 min cold) |
| `cargo test -p tinyquant-cli` | 3 / 3 smoke tests pass (debug, ~43 s) |
| Stripped release binary size (`tinyquant.exe`) | **2.49 MiB** (well under the 8 MiB §Step 11 budget) |
| `bash crates/tinyquant-cli/scripts/cli-smoke.sh` | end-to-end chain green (`info → train → compress → decompress → verify → search`) with `MSE ≈ 1.6e-9` on the pinned 1024×32 Gaussian dataset — ~6×10⁶ under the 1e-2 release-gate threshold declared in §CLI smoke test matrix |

The 8 MiB size budget in the §Step 11 spec is measured for
`linux-gnu-x64` with `strip = "symbols"` and PIE off. The Windows
MSVC PE format already drops debug info into a sibling `.pdb`, so
the `.exe` ships smaller than a stripped Linux ELF would. Strict
enforcement against the linux-gnu-x64 binary is deferred to the
Phase 22.D release-workflow runner, which will publish artefacts
for that target and gate the size budget there.

## Commit trail

| Commit | Summary |
|---|---|
| `978ebf1` | `feat(phase-22.A/py): pyo3 wheel with tinyquant_cpu parity surface` |
| `158a132` | `refactor(phase-22.A/py): address code-review feedback on parity suite and features` |
| `f1eae7c` | `feat(phase-22.B/sys): c abi handles, cbindgen header, and test surface` |
| *prior PR* | `fix(phase-22.B/sys): substitute CARGO_PKG_VERSION in generated header + const_assert` + `docs(rust/phase-22): implementation notes covering Parts A+B` |
| `85aa374` | `test(phase-22.C/cli): failing smoke matrix + Cargo scaffold` (TDD red) |
| *this PR* | `feat(phase-22.C/cli): standalone tinyquant binary with codec / corpus / verify subcommands` (TDD green + smoke scripts + impl notes) |

## Part D — Cross-architecture release pipeline

This section documents the Phase 22.D dry-run landing — the YAML,
Dockerfile, compatibility ledger, and xtask additions that make the
release surface ready to exercise against a pre-release tag without
publishing anything.

### Files landed

| File | Purpose |
|---|---|
| `rust/Dockerfile` | Two-stage builder + distroless `nonroot` runtime. `SOURCE_DATE_EPOCH` passed as `ARG`; base-image digests supplied via `ARG` sentinels. |
| `rust/.dockerignore` | Strips `target/`, `.git/`, Python reference, docs, and editor chaff from the build context. |
| `.github/workflows/rust-release.yml` | Seven-stage pipeline: gate → supply-chain-gate → matrix-sync → build → container-reproducibility → publish-{crates,pypi,container} → publish-release. |
| `COMPATIBILITY.md` (repo root) | First row of the `(tinyquant_cpu, tinyquant_rs)` ledger with the R19/R2 rotation-kernel drift called out. |
| `rust/crates/tinyquant-cli/README.md` | Crate-level README required by `cargo publish` because the manifest declares `readme = "README.md"`. |
| `rust/xtask/src/cmd/matrix_sync.rs` | `check-matrix-sync` subcommand asserting the CLI smoke matrix in the plan doc matches the `build.strategy.matrix.include` block in `rust-release.yml`. |
| `rust/xtask/src/main.rs` | Wires `bench-budget` (alias for `bench --check-against main`) and `check-matrix-sync` into the task dispatcher. |
| `rust/Cargo.toml` | Added `version = "=0.1.0"` to the `tinyquant-core`/`tinyquant-io`/`tinyquant-bruteforce` workspace-dependency entries so `cargo publish` accepts the manifest. `[profile.release]` was already configured in Phase 20 (`lto="fat"`, `codegen-units=1`, `strip=true`, `debug=0`). |

### Dockerfile pinning approach

The base images are **not** pinned to real SHA-256 digests in this
change — the Dockerfile carries two `ARG` sentinels
(`REPLACE_WITH_PINNED_DIGEST_RUST_1_81_BOOKWORM` /
`REPLACE_WITH_PINNED_DIGEST_DISTROLESS_CC_DEBIAN12_NONROOT`) that
the `container-reproducibility` job in `rust-release.yml` rejects
at build time via an explicit grep guard. Rationale:

1. This worktree has no sanctioned network access to resolve
   registry digests, and pinning to stale digests would guarantee a
   broken workflow by the time it runs.
2. The sentinel pattern lets the release operator resolve digests
   once (`docker buildx imagetools inspect rust:1.81-bookworm
   --format '{{.Manifest.Digest}}'` and the distroless equivalent)
   and commit the result as part of the release-prep PR, exercising
   the dry-run workflow before the real tag lands.
3. CI fails fast if the sentinels are still present — see the
   "Guard against placeholder digests" step.

### YAML structure decisions

- **Single publish guard.** Every publish stage carries the same
  `if:` expression
  (`startsWith(github.ref, 'refs/tags/rust-v') &&
    !contains(github.ref_name, '-alpha') &&
    !contains(github.ref_name, '-rc') &&
    inputs.dry_run != true`).
  The YAML anchor pattern can't be used across jobs, so the
  expression is duplicated literally; it is self-consistent and any
  one change must be mirrored to the other three jobs in the same
  edit. The "## Dry-run behavior" comment block at the top of the
  YAML documents this.
- **`workflow_dispatch` with `inputs.dry_run`.** Operators can run
  the workflow from the UI (`gh workflow run rust-release.yml`)
  against any ref; the input defaults to `true` so an accidental
  manual fire cannot publish. Combined with the tag guard, only a
  non-prerelease tag push (or a manual run with `dry_run: false`)
  can publish.
- **Seven stages, not six.** The spec lists six
  (gate / build / publish-crates / publish-pypi / publish-container
  / publish-release). This implementation adds two non-publishing
  stages: `supply-chain-gate` (cargo-vet + cargo-audit) and
  `matrix-sync` (xtask). Both depend on `gate` and gate the
  `build` matrix. The `container-reproducibility` job also runs
  between `build` and the publish stages and asserts the OCI image
  ID is identical across two independent buildx passes.
- **Attestations alongside artefacts.** `actions/attest-build-
  provenance@v1` is invoked twice per build job (once for the CLI
  archive, once for the wheel) and once on the container publish.
  The generated attestations go to the GitHub attestation store
  and the container attestation is also pushed to the registry via
  `push-to-registry: true`.
- **SBOM + cosign.** Every build job runs `syft` to emit a
  CycloneDX SBOM next to the archive and `cosign sign-blob` to
  produce a keyless OIDC signature. On Windows runners `syft` is
  skipped (the `anchore/sbom-action/download-syft@v0` action isn't
  published for Windows at the time of writing); the wheel
  attestation still runs.
- **Size budget enforcement.** Only the `x86_64-unknown-linux-gnu`
  leg enforces the 8 MiB binary size budget. The spec's budget is
  defined against stripped ELF; Windows PE and macOS Mach-O both
  report smaller sizes and are not the gate.

### xtask additions

- **`cargo xtask check-matrix-sync`.** Parses the CLI smoke-test
  matrix table out of
  `docs/plans/rust/phase-22-pyo3-cabi-release.md` (looking inside
  the `#### CLI smoke test matrix` section) and the
  `build.strategy.matrix.include` block of `rust-release.yml`, then
  diffs the two sets. Both sources currently agree on 9 targets
  (`x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`,
  `x86_64-unknown-linux-musl`, `aarch64-unknown-linux-musl`,
  `x86_64-apple-darwin`, `aarch64-apple-darwin`,
  `x86_64-pc-windows-msvc`, `i686-pc-windows-msvc`,
  `x86_64-unknown-freebsd`). Four unit tests cover the triple
  heuristic and the two extractors.
- **`cargo xtask bench-budget`.** Convenience alias for
  `cargo xtask bench --check-against main`. The underlying
  implementation in Phase 21's `cmd::bench` module already reads the
  locked budget from the committed `baselines/main.json` and
  compares it against a fresh criterion run, returning non-zero on
  budget exceedance — which is exactly the semantics the task
  brief asks for. The alias exists purely so the release workflow
  can call a single verb.

### Supply-chain wiring

The release workflow wires, in order:

1. **`cargo-vet` + `cargo-audit`** in `supply-chain-gate` (runs
   after `gate`, before `build`). `cargo vet` is
   `continue-on-error: true` because the audits.toml bootstrap
   has not yet been committed — the job still emits a JSON report.
   `cargo audit --deny warnings` is strict.
2. **Keyless cosign** via `sigstore/cosign-installer@v3`, signing
   each CLI archive as a blob and the container image via
   registry digest.
3. **SLSA build provenance** via
   `actions/attest-build-provenance@v1`, run three times per build
   (CLI archive, wheel, and in the container publish job).
4. **CycloneDX SBOM** via `anchore/sbom-action/download-syft@v0`.
   For blobs the `.cdx.json` is uploaded alongside the archive;
   for containers it is attached via `cosign attach sbom`.

### Dry-run verification performed locally

This worktree has no network access to trusted publishers, so the
dry-run is structural and host-local only.

| Gate | Result |
|---|---|
| `yamllint .github/workflows/rust-release.yml` (relaxed, commas/braces disabled) | clean |
| `npx @action-validator/cli` | clean except for `/permissions/attestations` — schema lag, not a real error; documented inline |
| `docker buildx build --check -f rust/Dockerfile --target builder .` | parses; fails at image pull because placeholder digests (expected — CI guard rejects sentinels explicitly) |
| `cargo publish --dry-run -p tinyquant-core --allow-dirty` | succeeds (`warning: aborting upload due to dry run`) |
| `cargo package --list -p tinyquant-io / -bruteforce / -pgvector / -sys / -cli` | all succeed; downstream crates cannot run full `--dry-run` until upstream crates exist on crates.io |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean |
| `cargo test --workspace` | green (all pass / ignored counts unchanged from Phase 22.C) |
| `cargo test -p xtask` | 9 / 9 pass (4 new `matrix_sync` tests) |
| `cargo run -p xtask -- check-matrix-sync` | `plan and release.yml agree on 9 targets ✓` |
| `cargo run -p xtask -- bench --validate` | baseline schema valid |
| `cargo build -p tinyquant-cli --release` binary size (Windows PE) | 2.21 MiB (well under the 8 MiB budget; linux-gnu-x86_64 enforced in CI) |

### Deferred / open items (Part D backlog)

- **Real base-image digests.** Left as `REPLACE_WITH_PINNED_DIGEST_*`
  sentinels. Must be resolved before any real release tag; the
  CI workflow rejects the sentinel at build time, so a dry-run
  tag will surface the missing pin without risk.
- **`cargo-vet` audits bootstrap.** `supply-chain/audits.toml` is
  not yet committed; the `cargo vet` step is
  `continue-on-error: true` until the initial audit set is
  published. Tracked as a Part-D follow-up.
- **`compatibility-check` xtask.** The spec mentions an
  `xtask compatibility-check` that reads `COMPATIBILITY.md` and
  asserts the top row matches the workspace version. Deferred —
  the ledger file exists but is reviewed manually during release
  prep for the 0.1.0 release.
- **Separate `rust-release-smoke.yml`.** The spec suggests an
  optional PR-smoke variant that runs the build matrix without
  tagging. Skipped because the `workflow_dispatch` input on
  `rust-release.yml` already allows the release workflow itself
  to be run in dry-run mode against any branch. A dedicated
  smoke workflow would duplicate ~80 % of the release YAML for
  marginal benefit.
- **PyPI + crates.io trusted-publisher setup.** Project-side
  configuration on pypi.org and crates.io cannot be done from
  inside this repo; the workflow's `id-token: write` scope and
  the `pypa/gh-action-pypi-publish@release/v1` action are ready
  for the switchover once the trusted publishers are registered.
- **FreeBSD round-trip smoke.** `cross` builds the CLI for
  `x86_64-unknown-freebsd` but the §CLI smoke test matrix only
  runs `info` / `verify` there (no runnable qemu emulator is
  pinned in `cross`' image). The YAML matrix already gates this
  via `matrix.target == 'x86_64-unknown-freebsd'`; full smoke is
  deferred.

### Part D spec-review follow-up

Spec reviewer on the first Part D pass flagged five undeclared
deviations from the spec. All five were closed in a follow-up
commit batch before code-quality review:

- **D1 — Dockerfile `--features tracing-json`.** Builder stage now
  compiles with `--features tracing-json` on top of the CLI
  default set so the container image emits structured logs by
  default, matching spec Step 18's sample invocation.
- **D2 — CLI README feature-flag table + `COMPATIBILITY.md`
  back-link.** `rust/crates/tinyquant-cli/README.md` now carries
  a `## Cargo features` table derived from the §CLI feature flag
  matrix plus a `## Compatibility` section linking back to the
  root ledger.
- **D3 — `cargo-auditable` before SBOM.** The `build` matrix now
  runs `cargo auditable build` on Linux targets before `syft`
  generates the CycloneDX SBOM, so downstream consumers can
  extract the full dependency audit trail with `cargo audit bin`.
  Restricted to Linux because the auditable toolchain is
  best-tested there and the SBOM leg already skips Windows.
- **D4 — `cli-smoke` per-target step.** The `build` matrix now
  invokes `rust/crates/tinyquant-cli/scripts/cli-smoke.{sh,ps1}`
  for every runnable target (`matrix.cross == false &&
  matrix.target != 'x86_64-unknown-freebsd'`), feeding
  `TINYQUANT_BIN` so each smoke run exercises the freshly built
  per-triple binary instead of a `target/release/` default.
- **D5 — `rewrite-timestamp` on `docker/build-push-action@v6`.**
  All three build-push invocations (`container-reproducibility`
  pass 1 + 2 and `publish-container`) now set
  `SOURCE_DATE_EPOCH` as a step-level env var and add
  `outputs: …,rewrite-timestamp=true` so BuildKit normalizes
  layer mtimes in the resulting OCI manifest. The Dockerfile
  reproducibility-contract comment block (§3) is updated to
  reflect the real wiring.

### Part D code-quality follow-up

The code-quality review of the Part D landing pass returned *Needs
changes* with three blockers and several nits. All are closed in a
follow-up commit batch on the same branch; notes below so future
reviewers can trace why the files look the way they do.

- **M1 — `CARGO_REGISTRY_TOKEN` was not wired.** The
  `publish-crates` job's `cargo publish` calls would have failed at
  the first real tag because no registry token was exported. Added a
  step-level `env: CARGO_REGISTRY_TOKEN: ${{ secrets.CARGO_REGISTRY_TOKEN }}`
  on the `Publish crates (ordered)` step and an inline comment
  documenting the required repo secret. **Operator action:**
  before cutting `rust-v0.1.0`, create a crates.io API token scoped
  to the `tinyquant-*` crate family (Account Settings → API Access
  on crates.io) and add it as the `CARGO_REGISTRY_TOKEN` secret
  under repo Settings → Secrets and variables → Actions. No CI
  rehearsal exists for this token — pre-release tags skip the
  publish job entirely (see M2 below).
- **M2 — publish-guard allowed most pre-release suffixes.** The
  original guard only excluded `-alpha` and `-rc`. Any tag with a
  different semver pre-release marker (`-beta`, `-pre`, `-preview`,
  numbered snapshots, `+build.*`) would have reached the real
  publish path. Tightened to the blanket
  `!contains(github.ref_name, '-') && !contains(github.ref_name, '+')`
  pair — semver 2.0 pre-release tokens start with `-` and build-
  metadata tokens start with `+`, so only clean release tags like
  `rust-v0.1.0` now trigger publish. Applied identically to all four
  `publish-*` jobs.
- **M3 — no enforcement of guard-block equality.** The four publish
  jobs' `if:` blocks must stay byte-identical but nothing asserted
  this (Lesson L3 drift hazard). Added
  `rust/xtask/src/cmd/guard_sync.rs`, wired into
  `cargo xtask check-matrix-sync` so a single CLI verb runs both
  the CLI-smoke-matrix check and the guard-equality check. The
  parser handles the `if: >-` folded-scalar style, whitespace-
  normalises the extracted expressions, and exits non-zero with a
  diff-style report on drift. Five new unit tests in
  `cmd::guard_sync::tests` cover the happy path, the drift path,
  job enumeration, filtering of non-publish jobs, and whitespace
  insensitivity. Chose the sibling-module layout (Option A from the
  review) over extending `matrix_sync.rs` inline because the two
  parsers share no logic and keeping them separate keeps each file
  independently testable.
- **N1 — Dockerfile comment referenced a nonexistent workflow step.**
  The reproducibility-contract comment claimed the base-image
  digest sentinels were resolved by a `resolve-base-digests` step
  that does not exist. Rewrote the comment to describe the real
  mechanism: the `Guard against placeholder digests` step in the
  `container-reproducibility` stage greps for the
  `REPLACE_WITH_PINNED_DIGEST` placeholder and fails the pipeline
  if it is still present; operators supply the real digest either
  by committing an edit or via `--build-arg`.
- **N2 — workflow header referenced a nonexistent YAML anchor.**
  The header comment advertised a `&publish_guard` anchor that was
  never declared (GitHub Actions does not reliably expand anchors
  across jobs, so the guard is duplicated verbatim). Reworked the
  header comment to describe the real duplication-plus-xtask-check
  shape — the text now points readers at
  `cargo xtask check-matrix-sync` as the equality enforcer.
- **N4 — `COMPATIBILITY.md` drift metric was mis-labelled.** Row 1
  of the ledger called the rotation-kernel drift `MSE` but the
  number (3.15e-4) is `max |py − rs|` per vector, measured against
  the 1e-3 "tight numerical parity" tolerance defined in
  `docs/design/rust/numerical-semantics.md`. Relabelled the cell;
  no other changes to the row.

Nit items N3 and the long-form review paragraphs outside the M/N set
are not addressed in this pass — the fix agent had a bounded file
scope and deeper refactors (e.g. collapsing the four `if:` blocks
into a reusable composite action) would leak outside that scope.
They are backlogged for the next Rust phase.

### Part D commit trail

| Commit | Summary |
|---|---|
| *this PR* | `feat(phase-22.D/release): rust-release.yml with build matrix and dry-run guards` |
| *this PR* | `feat(phase-22.D/release): container image with reproducibility gates` |
| *this PR* | `feat(phase-22.D/xtask): check-matrix-sync + bench-budget alias` |
| *this PR* | `docs(root): initial COMPATIBILITY.md` |
| *this PR* | `docs(rust/phase-22): implementation notes §Part D` |
| *this PR* | `fix(phase-22.D/release): close 5 spec-review deviations (tracing-json, cli-readme, cargo-auditable, cli-smoke, rewrite-timestamp)` |
| *this PR* | `fix(phase-22.D/release): wire CARGO_REGISTRY_TOKEN for crates.io publish` |
| *this PR* | `fix(phase-22.D/release): tighten publish guard to exclude all pre-release and build-metadata tags` |
| *this PR* | `feat(phase-22.D/xtask): enforce byte-identical publish-job guards` |
| *this PR* | `docs(rust/phase-22): correct Dockerfile digest-pin description and COMPATIBILITY drift label` |
| *this PR* | `docs(rust/phase-22): code-quality follow-up notes` |
