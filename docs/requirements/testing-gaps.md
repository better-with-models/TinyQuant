---
title: Testing Gaps — Requirements Without Adequate Test Coverage
tags:
  - requirements
  - testing
  - gaps
date-created: 2026-04-15
status: active
category: requirements
---

# Testing Gaps

Requirements that lack adequate test coverage, grouped by priority.
Each gap has a unique `GAP-*` tag that is referenced in the requirement
block where it appears.

## Priority definitions

| Priority | Meaning |
|---|---|
| **P0 — Blocker** | The requirement has no test at all; a regression would be invisible |
| **P1 — High** | A test exists in one implementation only (Python or Rust), not both |
| **P2 — Medium** | A test exists but does not enforce the quantitative Must threshold |
| **P3 — Low** | A test exists and the invariant is enforced structurally; gap is defense-in-depth |

---

## P0 — Blocker gaps

### GAP-BACK-004 — Batch error isolation (partial-success)

```
Requirement:  FR-BACK-004
Gap:          No test exercises the batch-mode partial-success scenario: a corpus
              of N vectors where M < N are corrupt, verifying that (N - M) good
              vectors are still delivered and all M corrupt vector_ids appear in
              the error report.
Current:      rejection.rs tests verify individual vector rejection. They do not
              test the aggregate behavior of a mixed-success batch.
Action:       Add Rust integration test in tinyquant-core/tests/corpus_aggregate.rs:
              inject M=1 corrupt VectorEntry into a batch of N=10; verify 9 FP32
              vectors returned AND error report contains the corrupt vector_id.
              Mirror with a Python test in tests/corpus/test_corpus.py.
```

### GAP-BACK-005 — Query vector passthrough behavioral test

```
Requirement:  FR-BACK-005
Gap:          No behavioral test intercepts the query vector at the backend
              interface and verifies element-wise equality to the caller-supplied
              vector. The invariant is enforced structurally (SearchBackend::search
              takes &[f32]) but structure does not substitute for a behavioral test.
Current:      backend_trait.rs::test_backend_trait_search_multiple_vectors exercises
              the search path but does not compare query vectors at both ends.
Action:       Add a test that wraps a BruteForceBackend with a spy adapter that
              records the query vector; assert it equals the original FP32 vector
              after calling corpus.search(query, backend).
```

### GAP-QUAL-004 — Top-10 neighbor overlap (Rust only)

```
Requirement:  FR-QUAL-004
Gap:          No Rust test measures top-10 neighbor overlap. The only test is
              Python-only (tests/calibration/test_score_fidelity.py::test_top10_overlap_4bit).
Current:      The Rust calibration suite (tinyquant-bench/tests/calibration.rs) does
              not include a top-k overlap metric at all.
Action:       Add an #[ignore] calibration test in tinyquant-bench/tests/calibration.rs
              measuring mean Jaccard overlap for 100 query vectors on the gold corpus.
              Must gate: ≥ 80%. Promote to non-ignored once CI runner provides the
              gold corpus fixture.
```

---

## P1 — High priority gaps

### GAP-COMP-006 — Dimension mismatch rejection (Rust end-to-end)

```
Requirement:  FR-COMP-006
Gap:          The Rust errors.rs test validates error display format only. No Rust
              integration test calls compress() with a mismatched dimension and
              verifies both the error type (CodecError::DimensionMismatch) and the
              absence of a CompressedVector output.
Current:      [Python] tests/codec/test_codec.py::TestCompress::test_compress_dimension_mismatch_raises ✅
              [Rust]   tinyquant-core/tests/errors.rs::dimension_mismatch_error_format (format only)
Action:       Add to tinyquant-core/tests/codec_service.rs:
                #[test]
                fn compress_dimension_mismatch_returns_error_and_no_output() {
                    let config = CodecConfig::new(4, 42, 768, true).unwrap();
                    let short_vector = vec![0f32; 512];
                    let result = Codec::new().compress(&short_vector, &config, &codebook);
                    assert!(matches!(result, Err(CodecError::DimensionMismatch { .. })));
                }
```

### GAP-COMP-007 — Config hash embedded in CompressedVector (Rust)

```
Requirement:  FR-COMP-007
Gap:          No Rust integration test asserts CV.config_hash == C.hash() after a
              full compress() call. The codec_config test verifies hash stability in
              isolation, not the end-to-end embedding.
Current:      [Python] tests/codec/test_codec.py::TestCompress::test_compress_config_hash_matches ✅
              [Rust]   tinyquant-core/tests/codec_config.rs::test_config_hash_is_deterministic (isolation)
Action:       Add to tinyquant-core/tests/codec_service.rs:
                #[test]
                fn compress_embeds_config_hash_in_output() {
                    let cv = Codec::new().compress(&vector, &config, &codebook).unwrap();
                    assert_eq!(cv.config_hash(), config.hash().as_ref());
                }
```

### GAP-DECOMP-003 — Config mismatch rejection at decompress (Rust end-to-end)

```
Requirement:  FR-DECOMP-003
Gap:          The Rust errors.rs test validates the error display format. No Rust
              integration test supplies a CV with one config hash and attempts
              decompress with a different config, verifying both the error type and
              the absence of output.
Current:      [Python] tests/codec/test_codec.py::TestDecompress::test_decompress_config_mismatch_raises ✅
              [Rust]   tinyquant-core/tests/errors.rs::config_mismatch_error (format only)
Action:       Add to tinyquant-core/tests/codec_service.rs:
                #[test]
                fn decompress_config_mismatch_returns_error_and_no_output() {
                    let cv = compress_with(config_a);
                    let result = Codec::new().decompress(&cv, &config_b, &codebook_b);
                    assert!(matches!(result, Err(CodecError::ConfigMismatch { .. })));
                }
```

### GAP-CORP-002 — CodecConfig immutability at Corpus level (Rust)

```
Requirement:  FR-CORP-002
Gap:          The Rust immutability tests cover VectorEntry, not the Corpus-level
              CodecConfig field. No Rust test attempts to replace the config on an
              existing Corpus.
Current:      [Python] tests/corpus/test_corpus.py::TestCreation::test_create_corpus_freezes_config ✅
              [Rust]   tinyquant-core/tests/vector_entry.rs::vector_entry_immutable_after_construction
              (covers VectorEntry, not Corpus config)
Action:       Add to tinyquant-core/tests/corpus_aggregate.rs:
                #[test]
                fn corpus_config_is_frozen_after_creation() {
                    // Attempt to replace corpus.config via any available API;
                    // assert CorpusError::Immutable or compile-time rejection.
                }
```

### GAP-CORP-007 — Policy immutability after first insertion (Rust confirmation)

```
Requirement:  FR-CORP-007
Gap:          No confirmed Rust test exercises the policy-immutability rejection on
              a populated corpus. The corpus_aggregate.rs tests cover many insertion
              scenarios but the specific case of a policy-change attempt on a
              populated corpus has not been verified against the test inventory.
Current:      [Python] tests/corpus/test_corpus.py (policy immutability, assumed present)
              [Rust]   corpus_aggregate.rs — not confirmed against inventory
Action:       Confirm or add in tinyquant-core/tests/corpus_aggregate.rs:
                #[test]
                fn policy_change_on_populated_corpus_is_rejected() {
                    // Insert one vector; attempt policy change;
                    // assert CorpusError::PolicyImmutable.
                }
```

### GAP-JS-004 — Corpus policy invariants not exercised through N-API boundary

```
Requirement:  FR-JS-004
Gap:          corpus.test.ts does not test cross-config insertion rejection
              (FR-CORP-003), FP16 policy precision bound (FR-CORP-006), or
              compression policy immutability after first insertion (FR-CORP-007).
              These invariants are tested in the Rust and Python layers but not
              through the tinyquant-js N-API binding.
Current:      [Rust]   tinyquant-core/tests/corpus_aggregate.rs (policy guards) ✅
              [Python] tests/corpus/test_corpus.py (policy guards) ✅
              [Node]   javascript/@tinyquant/core/tests/corpus.test.ts (lifecycle only)
Action:       Add to corpus.test.ts:
              (1) Insert a vector compressed with config A; attempt to insert one
                  with config B; assert TinyQuantError with code "ConfigMismatchError".
              (2) After one insertion, attempt to change compressionPolicy; assert
                  TinyQuantError with code "PolicyImmutableError".
              (3) FP16 policy: insert 100 vectors; decompressAll; measure element-wise
                  absolute difference against originals; assert max delta ≤ 2^-13 x |v|.
```

### GAP-BACK-003 — pgvector adapter dimensionality (offline test)

```
Requirement:  FR-BACK-003
Gap:          The pgvector adapter dimensionality test requires a live Docker container
              and is excluded from standard CI. No offline/unit test covers the
              dimensionality preservation guarantee.
Current:      [Rust] tinyquant-pgvector/tests/adapter.rs (live-db gated, not in standard CI)
Action:       Extract the dimensionality assertion into a unit test that mocks the
              pgvector wire format without a real database:
                #[test]
                fn pgvector_adapter_preserves_dimension() {
                    let formatted = format_as_pgvector(&fp32_vector);
                    let recovered = parse_pgvector(&formatted);
                    assert_eq!(fp32_vector.len(), recovered.len());
                    // element-wise float comparison
                }
```

---

## P2 — Medium priority gaps

### GAP-COMP-004 — Residual length check in compress (Rust)

```
Requirement:  FR-COMP-004
Gap:          No Rust integration test verifies that CV.residual.len() == dim * 2
              after a full compress() call with residual_enabled true. The unit test
              (compressed_vector.rs::new_with_residual_payload) constructs the type
              directly, bypassing the compress path.
Current:      [Python] tests/codec/test_codec.py::TestCompress::test_compress_with_residual ✅
              [Rust]   tinyquant-core/tests/compressed_vector.rs::new_with_residual_payload (unit, not integration)
Action:       Add to tinyquant-core/tests/codec_service.rs:
                fn compress_with_residual_sets_correct_payload_length() {
                    let cv = compress_with(residual_enabled: true);
                    assert_eq!(cv.residual().unwrap().len(), config.dim() as usize * 2);
                }
```

### GAP-DECOMP-004 — Residual improves MSE (Rust non-ignored)

```
Requirement:  FR-DECOMP-004
Gap:          No non-ignored Rust test computes and compares MSE for residual-on vs
              residual-off on the same vectors. The calibration tests that exercise
              this are all #[ignore].
Current:      [Python] tests/codec/test_codec.py::TestRoundTrip::test_round_trip_with_residual_has_lower_error ✅
              [Python] tests/calibration/test_score_fidelity.py::test_residual_improves_rho ✅
              [Rust]   tinyquant-core/tests/residual.rs::residual_round_trip (round-trip only, not MSE comparison)
              [Rust]   tinyquant-bench/tests/calibration.rs::* (all #[ignore])
Action:       Add a lightweight (< 1 s) MSE comparison test to
              tinyquant-core/tests/codec_service.rs using the dim=64 fixture:
                #[test]
                fn residual_on_has_lower_mse_than_residual_off() {
                    // compress + decompress with and without residual;
                    // compute MSE; assert mse_on < mse_off for ≥ 95% of 100 test vectors
                }
              This is feasible without the gold corpus because dim=64 fixtures are
              already in LFS and the test runs in < 100 ms.
```

### GAP-CORP-001 — Corpus ID uniqueness

```
Requirement:  FR-CORP-001
Gap:          No test asserts that two Corpus instances created in the same process
              have distinct corpus_id values. Existing tests verify that a corpus
              has an id; none verify uniqueness across instances.
Current:      [Rust]   tinyquant-core/tests/corpus_aggregate.rs::create_corpus_begins_empty (exists, id checked)
              [Python] tests/corpus/test_corpus.py::TestCreation::test_create_empty_corpus (id checked)
Action:       Add to tinyquant-core/tests/corpus_aggregate.rs:
                #[test]
                fn each_corpus_gets_unique_id() {
                    let ids: Vec<_> = (0..100).map(|_| Corpus::new(...).id()).collect();
                    let unique: std::collections::HashSet<_> = ids.iter().collect();
                    assert_eq!(ids.len(), unique.len());
                }
```

### GAP-CORP-006 — FP16 precision bound

```
Requirement:  FR-CORP-006
Gap:          The Must threshold (|FP16-cast − original| ≤ 2^-13 × |original| per
              element) is not measured in any test. The residual_encodes_fp16 test
              verifies encoding format, not the precision error bound.
Current:      [Rust]   tinyquant-core/tests/residual.rs::residual_encodes_fp16 (format only)
Action:       Add to tinyquant-core/tests/compression_policy.rs:
                #[test]
                fn fp16_policy_precision_within_bound() {
                    let original: Vec<f32> = ...;
                    let retrieved = round_trip_via_fp16_policy(&original);
                    for (o, r) in original.iter().zip(retrieved.iter()) {
                        let bound = 2f32.powi(-13) * o.abs();
                        assert!((o - r).abs() <= bound, "element exceeds FP16 precision bound");
                    }
                }
```

### GAP-SER-003 — Zero-copy heap measurement

```
Requirement:  FR-SER-003
Gap:          The zero_copy tests assert structural no-allocation behavior but do not
              instrument the heap to enforce the ≤ 4 KiB per 1 000 vectors Must
              threshold.
Current:      [Rust]   tinyquant-io/tests/zero_copy.rs::zero_copy_slice_indices_no_allocation ✅ (structural)
              [Rust]   tinyquant-io/tests/mmap_corpus.rs::mmap_corpus_memory_mapped_not_copied ✅ (structural)
Action:       The existing test-mmap-dhat CI job (rust-ci.yml) runs:
                cargo test -p tinyquant-io --features "mmap dhat-heap" --test zero_copy
              Extend the dhat-heap test to assert peak_heap_delta ≤ 4096 bytes per
              1 000 vectors by reading the dhat output file and asserting the heap
              increment bound.
```

### GAP-JS-002 — Round-trip test covers dim=128 only

```
Requirement:  FR-JS-002
Gap:          round-trip.test.ts tests N=10 000, dim=128 only. No JS test covers
              dim=768 (the production gold corpus dimension) or dim=1536 (the Rust
              perf target), so a codec regression at higher dimension would be
              invisible in the Node package.
Current:      [Node]   javascript/@tinyquant/core/tests/round-trip.test.ts (dim=128 only)
              [Rust]   tinyquant-bench/benches/compress.rs (dim=1536) ✅
              [Python] tests/calibration/test_score_fidelity.py (dim=768) ✅
Action:       Add a second it() block in round-trip.test.ts:
                N=1000, dim=768, bit_width=4, residual=false, seed 0xdeadbeef,
                codebook trained on first 256 vectors; assert MSE < 1e-2.
```

### GAP-JS-006 — musl Linux binary not bundled

```
Requirement:  FR-JS-006
Gap:          The loader correctly maps linux + musl → "linux-x64-musl" / "linux-arm64-musl"
              binary keys, but no corresponding .node files are bundled. Alpine Linux
              users receive the missing-binary error (a clear message, not a crash), but
              they cannot use the package.
Current:      [Node]   loader.test.ts (unit tests covering 5 glibc platforms) ✅
              Missing:  linux-x64-musl.node, linux-arm64-musl.node
Action:       Add musl cross-compilation to the napi build matrix:
                napi build --platform --release --target x86_64-unknown-linux-musl
              Wire as a new CI matrix cell in js-ci.yml. Coordinate with Phase 27
              (GPU crates also need a musl-clean build).
```

### GAP-JS-007 — Sub-path ESM exports not tested

```
Requirement:  FR-JS-007
Gap:          No test imports from the sub-path exports (/codec, /corpus, /backend).
              The test suite imports from the package root only. A broken sub-path
              export entry in package.json would go undetected.
Current:      [Node]   tests/round-trip.test.ts, parity.test.ts (root "." export only)
Action:       Add a smoke test that imports CodecConfig from "@.../tinyquant-core/codec",
              Corpus from "/corpus", and BruteForceBackend from "/backend"; assert each
              is a constructor. One new test file (esm-subpath-smoke.test.ts) is sufficient.
```

### GAP-JS-008 — No CI package-size gate

```
Requirement:  FR-JS-008
Gap:          No CI step runs `npm pack --dry-run` and asserts the gzipped and unpacked
              size thresholds. Adding a musl binary or a second architecture would push
              the package toward the 10 MB / 50 MB budget silently.
Current:      Size documented in Phase 25 plan (~4 MB gzip / ~8.5 MB uncompressed) but
              not automatically enforced.
Action:       Add to js-ci.yml a step after `npm run build`:
                npm pack --dry-run 2>&1 | grep -E "package size|unpacked size"
              Parse the output and assert gzip < 10 MB AND unpacked < 50 MB.
              Fail the job if either threshold is exceeded.
```

### GAP-JS-009 — Version consistency check runs at release only, not on PRs

```
Requirement:  FR-JS-009
Gap:          verify-version is a step in js-release.yml (triggered by v* tags). It does
              not run on PRs or pushes to main, so a PR that bumps package.json but not
              Cargo.toml (or vice versa) would merge cleanly and only fail at release.
Current:      [CI]  .github/workflows/js-release.yml::verify-version (release only)
Action:       Extract the version comparison logic into a reusable composite action or
              script (scripts/check_version_consistency.sh); call it in js-ci.yml on
              every PR that touches package.json or Cargo.toml.
```

### GAP-JS-010 — No-subprocess loader check may not be wired in CI

```
Requirement:  FR-JS-010
Gap:          scripts/check_no_exec.sh is documented in the Phase 25 plan as a CI step
              in js-ci.yml, but it is not confirmed that the step was added during
              Phase 25 implementation.
Current:      [CI]  scripts/check_no_exec.sh (documented, wiring unverified)
Action:       Verify that js-ci.yml contains a step calling check_no_exec.sh. If absent,
              add:
                - name: Verify loader has no subprocess invocations
                  run: bash scripts/check_no_exec.sh javascript/@tinyquant/core/dist
              Run this step after the build step so it checks the compiled output.
```

### GAP-QUAL-001 through GAP-QUAL-003 — Rust calibration gates are #[ignore]

```
Requirements: FR-QUAL-001, FR-QUAL-002, FR-QUAL-003, FR-QUAL-005, FR-QUAL-007, FR-QUAL-008
Gap:          All Rust Pearson ρ and compression ratio tests in
              tinyquant-bench/tests/calibration.rs are #[ignore]. They exist and
              pass locally but are not blocking CI gates. The comment in rust-ci.yml
              explains why (calibration jobs removed from the matrix in Phase 21).
Current:      [Python] tests/calibration/test_score_fidelity.py ✅ (active Python gates)
              [Rust]   tinyquant-bench/tests/calibration.rs (all #[ignore])
Action:       No new tests needed. The roadmap calls for restoring the Rust calibration
              gates in Phase 26 once the residual encoder improves the compression ratio
              to meet the planned thresholds. Until then, the Python calibration tests
              serve as the binding quality gate.
              Track in docs/plans/rust/phase-26-*.md.
```

---

## P3 — Low priority gaps

### GAP-BACK-001 — FP32 boundary architectural test

```
Requirement:  FR-BACK-001
Gap:          No test explicitly inspects the SearchBackend trait's method signatures
              to verify that no method accepts a CompressedVector. The trait design
              enforces this structurally (compile-time), but there is no behavioral
              test that confirms the invariant holds across all adapter implementations.
Current:      [Rust]   tinyquant-core/tests/backend_trait.rs::test_backend_trait_get_vector_accepts_valid_vector_ids
Action:       Add an architecture test (mirroring tests/architecture/ in Python) that
              reads the SearchBackend trait definition via Cargo metadata or source
              inspection and asserts no method parameter is of type CompressedVector.
              Alternatively, the compile-time guarantee is sufficient if documented.
```

### GAP-GPU-001 — cargo tree grep for GPU deps in tinyquant-core

```
Requirement:  FR-GPU-001
Gap:          The no_std CI job catches GPU leakage indirectly (a wgpu dep would
              break the no_std build). A direct grep of `cargo tree -p tinyquant-core`
              for GPU-related crates is not yet automated.
Current:      [CI]  rust-ci.yml::no-std-check (indirect; fails if GPU dep leaks)
Action:       Add to xtask simd audit or a new xtask gpu-isolation-check:
                cargo tree -p tinyquant-core --no-default-features \
                  | grep -E "wgpu|cust|gpu" && exit 1 || exit 0
              Wire as a CI step on PRs touching crates/tinyquant-gpu-*.
              Target phase: Phase 27 prerequisite.
```

### GAP-GPU-002 through GAP-GPU-007 — GPU crates not yet implemented

```
Requirements: FR-GPU-002 through FR-GPU-007
Gap:          All GPU requirements lack tests because the GPU crates (tinyquant-gpu-wgpu,
              tinyquant-gpu-cuda) are not yet implemented.
Current:      None.
Action:       Tests are planned in docs/design/rust/testing-strategy.md §GPU testing
              strategy. Implementation begins in Phase 27. Requirement stubs are tracked
              in docs/requirements/gpu.md.
```

---

## Gap summary table

| Tag | Requirement | Priority | Status |
|---|---|---|---|
| GAP-BACK-004 | Batch error isolation (partial-success) | **P0** | No test |
| GAP-BACK-005 | Query vector passthrough behavioral | **P0** | No behavioral test |
| GAP-QUAL-004 | Top-10 neighbor overlap (Rust) | **P0** | Python only |
| GAP-COMP-006 | Dimension mismatch rejection (Rust e2e) | **P1** | Python only |
| GAP-COMP-007 | Config hash embedded (Rust e2e) | **P1** | Python only |
| GAP-DECOMP-003 | Config mismatch at decompress (Rust e2e) | **P1** | Python only |
| GAP-CORP-002 | CodecConfig frozen at Corpus level (Rust) | **P1** | Python only |
| GAP-CORP-007 | Policy immutability (Rust confirmation) | **P1** | Unconfirmed |
| GAP-BACK-003 | pgvector dimensionality (offline) | **P1** | Live-db gated |
| GAP-JS-004 | Corpus policy invariants via N-API | **P1** | Rust/Python only |
| GAP-COMP-004 | Residual length in compress (Rust) | **P2** | Unit ≠ integration |
| GAP-DECOMP-004 | Residual improves MSE (Rust non-ignored) | **P2** | All #[ignore] |
| GAP-CORP-001 | Corpus ID uniqueness | **P2** | Existence checked, not uniqueness |
| GAP-CORP-006 | FP16 precision bound | **P2** | Format checked, not bound |
| GAP-SER-003 | Zero-copy heap measurement | **P2** | Structural, not measured |
| GAP-QUAL-001–003,005,007,008 | Rust calibration gates #[ignore] | **P2** | Python gates active |
| GAP-JS-002 | Round-trip test dim=128 only | **P2** | dim=768/1536 not covered |
| GAP-JS-006 | musl Linux binary not bundled | **P2** | Loader handles gracefully |
| GAP-JS-007 | Sub-path ESM exports not tested | **P2** | Root export only |
| GAP-JS-008 | No CI package-size gate | **P2** | Budget met but unguarded |
| GAP-JS-009 | Version check release-only, not PRs | **P2** | Release workflow only |
| GAP-JS-010 | No-subprocess check wiring unverified | **P2** | Documented, not confirmed |
| GAP-BACK-001 | FP32 boundary architectural | **P3** | Structurally enforced |
| GAP-GPU-001 | cargo tree grep for GPU deps | **P3** | Indirect enforcement only |
| GAP-GPU-002–007 | GPU crates not implemented | **P3** | Phase 27+ planned |

## Closing-phase assignment

Each gap is assigned to a roadmap phase. See [[roadmap|Roadmap]] §Gap remediation plan for the full rationale and per-phase scope.

| Gap | Closing phase |
|---|---|
| GAP-BACK-004, GAP-BACK-005 | Phase 21.5 |
| GAP-QUAL-004 | Phase 26 |
| GAP-COMP-006, GAP-COMP-007, GAP-DECOMP-003 | Phase 21.5 |
| GAP-CORP-002, GAP-CORP-007 | Phase 21.5 |
| GAP-BACK-003 | Phase 21.5 |
| GAP-JS-004 | Phase 25.5 |
| GAP-COMP-004, GAP-CORP-001, GAP-CORP-006, GAP-SER-003 | Phase 21.5 |
| GAP-DECOMP-004 | Phase 26 |
| GAP-QUAL-001–003,005,007,008 | Phase 26 |
| GAP-JS-002, GAP-JS-006, GAP-JS-007, GAP-JS-008, GAP-JS-009, GAP-JS-010 | Phase 25.5 |
| GAP-BACK-001 | Phase 21.5 |
| GAP-GPU-001 | Phase 27 |
| GAP-GPU-002–007 | Phases 27–28 |

## See also

- [[requirements/README|Requirements Overview]]
- [[roadmap|Roadmap]] §Gap remediation plan
- [[design/rust/testing-strategy|Testing Strategy]]
- [[design/rust/risks-and-mitigations|Risks and Mitigations]]
