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
