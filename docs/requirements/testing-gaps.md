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
Gap:          Test body exists but remains #[ignore] — gold corpus fixture helpers
              not yet provisioned in CI. Promote to non-ignored in the same PR
              that provisions the corpus LFS object (Phase 28).
Current:      [Python] tests/calibration/test_score_fidelity.py::test_top10_overlap_4bit ✅
              [Rust]   tinyquant-bench/tests/calibration.rs::jaccard_top10_overlap_4bit
                       (Phase 26 stub: #[ignore], Must gate ≥ 80% mean Jaccard)
Action:       Remove #[ignore] in the same PR that provisions the corpus LFS object.
              Phase 28.
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
Gap:          None. Closed in Phase 25.5.
Current:      [Rust]   tinyquant-core/tests/corpus_aggregate.rs (policy guards) ✅
              [Python] tests/corpus/test_corpus.py (policy guards) ✅
              [Node]   javascript/@tinyquant/core/tests/corpus.test.ts ✅
                       (1) config-hash distinctness + VectorEntry hash via N-API
                       (2) compressionPolicy stability after insertion via N-API
                       (3) FP16 round-trip precision (≤ 2^-10 × |v|) via N-API
              Note:    Full cross-config insertion rejection requires insertCompressed
                       to be exposed by the N-API binding (pending future phase).
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
Gap:          None. Closed Phase 26.
Current:      [Python] tests/codec/test_codec.py::TestRoundTrip::test_round_trip_with_residual_has_lower_error ✅
              [Python] tests/calibration/test_score_fidelity.py::test_residual_improves_rho ✅
              [Rust]   tinyquant-core/tests/residual.rs::residual_round_trip ✅
              [Rust]   tinyquant-core/tests/codec_service.rs::residual_on_has_lower_mse_than_residual_off ✅
                       (dim=64, 100 vectors, ≥ 95% must have lower MSE with residual — runs in < 100 ms)
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
Gap:          None. Closed in Phase 25.5.
Current:      [Node]   javascript/@tinyquant/core/tests/round-trip.test.ts ✅
                       (a) N=10k, dim=128 (Phase 25)
                       (b) N=1000, dim=768, seed 0xdeadbeef (Phase 25.5)
              [Rust]   tinyquant-bench/benches/compress.rs (dim=1536) ✅
              [Python] tests/calibration/test_score_fidelity.py (dim=768) ✅
```

### GAP-JS-006 — musl Linux binary not bundled

```
Requirement:  FR-JS-006
Gap:          None. Closed in Phase 25.5.
Current:      [CI]  js-ci.yml build-native matrix includes x86_64-unknown-linux-musl
                    and aarch64-unknown-linux-musl cells (musl-tools + gcc-aarch64 installed)
              [Node] javascript/@tinyquant/core/tests/loader.test.ts ✅
                    covers 5 glibc + 2 musl platform key computations
              [Code] binaryKey() accepts optional override params for unit testing
```

### GAP-JS-007 — Sub-path ESM exports not tested

```
Requirement:  FR-JS-007
Gap:          None. Closed in Phase 25.5.
Current:      [Node]   tests/esm-subpath-smoke.test.ts ✅
                       imports from dist/codec.js, dist/corpus.js, dist/backend.js;
                       asserts CodecConfig, Corpus, BruteForceBackend are constructors
              [Config] package.json "exports" extended with ./codec, ./corpus, ./backend
                       (ESM + CJS + types for each sub-path)
```

### GAP-JS-008 — No CI package-size gate

```
Requirement:  FR-JS-008
Gap:          None. Closed in Phase 25.5.
Current:      [CI]  assemble-tarball job: "Check package size" step uses
                    `npm pack --dry-run`; asserts gzip ≤ 10 MB and unpacked ≤ 50 MB.
                    Runs after all 7 binaries (5 glibc + 2 musl) are assembled.
```

### GAP-JS-009 — Version consistency check runs at release only, not on PRs

```
Requirement:  FR-JS-009
Gap:          None. Closed in Phase 25.5.
Current:      [CI]  .github/workflows/js-release.yml::verify-version (release) ✅
              [CI]  scripts/check_version_consistency.sh called in test-source job ✅
                    Compares package.json version vs Cargo.toml workspace version;
                    fails fast before npm ci on any PR touching the JS or Rust paths.
```

### GAP-JS-010 — No-subprocess loader check may not be wired in CI

```
Requirement:  FR-JS-010
Gap:          None. Closed in Phase 25.5.
Current:      [CI]  scripts/check_no_exec.sh created and called in test-source job ✅
                    Runs after npm test (dist/ is built); confirms no child_process,
                    spawnSync, execSync, or execFileSync patterns in compiled output.
```

### GAP-QUAL-001 through GAP-QUAL-003 — Rust calibration gates are #[ignore]

```
Requirements: FR-QUAL-001, FR-QUAL-002, FR-QUAL-003, FR-QUAL-005, FR-QUAL-007, FR-QUAL-008
Gap:          None. Closed Phase 26.
Current:      [Python] tests/calibration/test_score_fidelity.py ✅
              [Rust]   tinyquant-bench/tests/calibration.rs ✅
                       Five PR-speed gates (pr_speed_bw*) de-ignored in Phase 26;
                       run automatically in `cargo test --workspace`.
                       Full-corpus gates (openai_10k_d1536) remain #[ignore] until
                       Phase 28 provisions a dedicated CI calibration job.
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
Gap:          None. Closed Phase 26.
Current:      [CI]  rust-ci.yml::no-std-check (indirect; fails if GPU dep leaks)
              [CI]  rust-ci.yml::test — "Verify tinyquant-core has no GPU dependencies"
                    step added (Phase 26). Runs before cargo test --workspace.
                    Checks for \bwgpu\b|\bcust\b|\btinyquant-gpu\b in cargo tree output.
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
| GAP-QUAL-004 | Top-10 neighbor overlap (Rust) | **P0** | Phase 26 stub (`#[ignore]`); Phase 28 to un-ignore |
| GAP-COMP-006 | Dimension mismatch rejection (Rust e2e) | **P1** | Python only |
| GAP-COMP-007 | Config hash embedded (Rust e2e) | **P1** | Python only |
| GAP-DECOMP-003 | Config mismatch at decompress (Rust e2e) | **P1** | Python only |
| GAP-CORP-002 | CodecConfig frozen at Corpus level (Rust) | **P1** | Python only |
| GAP-CORP-007 | Policy immutability (Rust confirmation) | **P1** | Unconfirmed |
| GAP-BACK-003 | pgvector dimensionality (offline) | **P1** | Live-db gated |
| ~~GAP-JS-004~~ | ~~Corpus policy invariants via N-API~~ | ~~**P1**~~ | Closed Phase 25.5 |
| GAP-COMP-004 | Residual length in compress (Rust) | **P2** | Unit ≠ integration |
| ~~GAP-DECOMP-004~~ | ~~Residual improves MSE (Rust non-ignored)~~ | ~~**P2**~~ | Closed Phase 26 |
| GAP-CORP-001 | Corpus ID uniqueness | **P2** | Existence checked, not uniqueness |
| GAP-CORP-006 | FP16 precision bound | **P2** | Format checked, not bound |
| GAP-SER-003 | Zero-copy heap measurement | **P2** | Structural, not measured |
| ~~GAP-QUAL-001–003,005,007,008~~ | ~~Rust calibration gates #[ignore]~~ | ~~**P2**~~ | Closed Phase 26 |
| ~~GAP-JS-002~~ | ~~Round-trip test dim=128 only~~ | ~~**P2**~~ | Closed Phase 25.5 |
| ~~GAP-JS-006~~ | ~~musl Linux binary not bundled~~ | ~~**P2**~~ | Closed Phase 25.5 |
| ~~GAP-JS-007~~ | ~~Sub-path ESM exports not tested~~ | ~~**P2**~~ | Closed Phase 25.5 |
| ~~GAP-JS-008~~ | ~~No CI package-size gate~~ | ~~**P2**~~ | Closed Phase 25.5 |
| ~~GAP-JS-009~~ | ~~Version check release-only, not PRs~~ | ~~**P2**~~ | Closed Phase 25.5 |
| ~~GAP-JS-010~~ | ~~No-subprocess check wiring unverified~~ | ~~**P2**~~ | Closed Phase 25.5 |
| GAP-BACK-001 | FP32 boundary architectural | **P3** | Structurally enforced |
| ~~GAP-GPU-001~~ | ~~cargo tree grep for GPU deps~~ | ~~**P3**~~ | Closed Phase 26 |
| GAP-GPU-002–007 | GPU crates not implemented | **P3** | Phase 27+ planned |

## Closing-phase assignment

Each gap is assigned to a roadmap phase. See [[roadmap|Roadmap]] §Gap remediation plan for the full rationale and per-phase scope.

| Gap | Closing phase |
|---|---|
| GAP-BACK-004, GAP-BACK-005 | Phase 21.5 |
| GAP-QUAL-004 | Phase 28 |
| GAP-COMP-006, GAP-COMP-007, GAP-DECOMP-003 | Phase 21.5 |
| GAP-CORP-002, GAP-CORP-007 | Phase 21.5 |
| GAP-BACK-003 | Phase 21.5 |
| GAP-JS-004 | Phase 25.5 |
| GAP-COMP-004, GAP-CORP-001, GAP-CORP-006, GAP-SER-003 | Phase 21.5 |
| GAP-DECOMP-004 | Phase 26 |
| GAP-QUAL-001–003,005,007,008 | Phase 26 |
| GAP-GPU-001 | Phase 26 |
| GAP-JS-002, GAP-JS-006, GAP-JS-007, GAP-JS-008, GAP-JS-009, GAP-JS-010 | Phase 25.5 |
| GAP-BACK-001 | Phase 21.5 |
| GAP-GPU-002–007 | Phases 27–28 |

## See also

- [[requirements/README|Requirements Overview]]
- [[roadmap|Roadmap]] §Gap remediation plan
- [[design/rust/testing-strategy|Testing Strategy]]
- [[design/rust/risks-and-mitigations|Risks and Mitigations]]
