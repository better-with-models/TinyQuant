---
title: GPU Acceleration Requirements
tags:
  - requirements
  - planguage
  - gpu
date-created: 2026-04-15
status: draft
category: requirements
---

# GPU Acceleration Requirements

Functional and performance requirements for the optional GPU acceleration
layer (`tinyquant-gpu-wgpu` and `tinyquant-gpu-cuda`). These requirements
apply only when a compatible GPU adapter is available. The CPU path has no
dependency on these crates and must always function independently.

> [!note] Phase gating
> FR-GPU-001 through FR-GPU-004 gate Phase 27 acceptance.
> FR-GPU-005 through FR-GPU-007 gate Phase 28 acceptance.

---

### FR-GPU-001 — `tinyquant-core` remains CPU-only and `no_std`

```
Gist:       Adding GPU crates must not cause tinyquant-core to acquire any
            GPU or std dependency.
Type:       Constraint
Actor:      CI MSRV gate, no_std build job
Function:   `cargo tree -p tinyquant-core --no-default-features` shall
            contain no reference to `wgpu`, `cust`, `tinyquant-gpu-wgpu`,
            or `tinyquant-gpu-cuda`. The `thumbv7em-none-eabihf` no_std
            build shall continue to succeed.
Scale:      Presence of any GPU crate in the tinyquant-core dependency tree.
Meter:      CI job `no-std-check` runs
            `cargo build -p tinyquant-core --no-default-features
            --target thumbv7em-none-eabihf`.
            Separately, `cargo tree -p tinyquant-core` is grepped for
            "wgpu\|cust\|gpu".
Must:       0 GPU dependency references in tinyquant-core dep tree AND
            no_std build exits 0.
Plan:       Same.
Qualify:    GPU crates are workspace members (Phase 27+).
Rationale:  The core deployability promise is "runs in any container, no
            drivers required." That promise lives in tinyquant-core.
            Breaking no_std would also eliminate embedded and WASM targets.
Ref:        [[design/rust/gpu-acceleration]] §Core invariants
Tests:      [CI]     .github/workflows/rust-ci.yml::no-std-check
            (`cargo build -p tinyquant-core --no-default-features --target thumbv7em-none-eabihf`)
Gap:        GAP-GPU-001 — The `cargo tree` grep for "wgpu|cust|gpu" is not yet wired
            as a CI step; only the no_std build job currently exists. P2: build fails
            if a GPU dep leaks, but the tree audit is advisory only.
            See testing-gaps.md §GAP-GPU-001.
```

---

### FR-GPU-002 — GPU backend degrades gracefully when no adapter is present

```
Gist:       When no GPU adapter is available, the WgpuBackend signals
            unavailability and the caller falls back to CPU without error.
Type:       Function
Actor:      Library consumer, server process on headless machine
Function:   WgpuBackend::new() shall succeed (return Ok or an initialized
            struct) even when no compatible GPU adapter is found.
            is_available() shall return false in that case.
            No panic, no process exit, no unhandled error shall occur.
Scale:      Fraction of WgpuBackend::new() calls on adapter-less machines
            that return without error AND where is_available() == false.
Meter:      Run test suite on a standard CI runner (ubuntu-22.04, no GPU)
            with the validate-shaders feature; confirm no panics and
            is_available() == false.
Must:       100% of adapter-less new() calls return without error.
Plan:       Same.
Qualify:    No wgpu-compatible adapter (GPU or software fallback) present.
Rationale:  GPU is an additive feature; its absence must never crash a
            production server. Operators on CPU-only infrastructure should
            not need to change their deployment config.
Ref:        [[design/rust/risks-and-mitigations]] §R24
Tests:      [Crate]  tinyquant-gpu-wgpu/tests/context_probe.rs::wgpu_backend_new_does_not_panic
Gap:        None.
```

---

### FR-GPU-003 — GPU and CPU produce numerically equivalent results

```
Gist:       GPU compress_batch and CPU compress_batch produce results within
            a defined element-wise tolerance on the same input.
Type:       Constraint
Actor:      Differential testing harness
Function:   For any batch of N FP32 vectors V, GPU compress → decompress
            shall produce FP32 reconstructions R_gpu such that
            max |R_gpu[i][j] - R_cpu[i][j]| ≤ tolerance for all
            i in [0, N) and j in [0, dim).
Scale:      Max element-wise absolute difference between GPU and CPU
            decompressed output on the same input batch.
Meter:      Differential test (tinyquant-gpu-wgpu/tests/differential.rs):
            compress_batch then decompress_batch on 512 dim-768 vectors
            using GPU (llvmpipe software renderer) and CPU; record max delta.
Must:       Max delta ≤ 1e-3 (f32; accumulation difference from GPU matmul)
Plan:       Max delta ≤ 5e-4
Qualify:    GPU adapter available (physical or llvmpipe software renderer).
            Same CodecConfig and Codebook for both paths.
Rationale:  The GPU path is a performance optimization; it must not produce
            qualitatively different similarity rankings. Tolerance is wider
            than CPU determinism (1e-3 vs 0) because GPU FP32 GEMM can
            differ in reduction order.
Ref:        FR-COMP-003, [[design/rust/gpu-acceleration]] §Synchronization model
Tests:      [Crate]  tinyquant-gpu-wgpu/tests/parity_compress.rs::gpu_decompress_matches_cpu_within_tolerance
Gap:        None.
```

---

### FR-GPU-004 — GPU batch compress throughput (Phase 27 target)

```
Gist:       GPU batch compression of 10 000 vectors (dim 1536) completes
            in ≤ 5 ms on a discrete GPU adapter.
Type:       Performance
Actor:      Library consumer (GPU batch path)
Scale:      Wall-clock elapsed time in ms for GPU compress_batch(10 000
            vectors, dim 1536), including host↔device transfer.
Meter:      Median of 20 trials on a self-hosted runner with a discrete GPU
            (NVIDIA RTX 3060 class or better, or Apple M-series GPU).
            Batch must have been preceded by prepare_for_device() so device
            buffers are allocated (warm path).
Must:       Median ≤ 5 ms (380× faster than Python reference)
Plan:       Median ≤ 3 ms
Stretch:    Median ≤ 2 ms (GPU stretch from goals-and-non-goals)
Past:       CPU Rust target: ≤ 80 ms (FR-PERF-003)
Qualify:    Physical discrete GPU present; warm device buffers (after
            prepare_for_device()); dim 1536; bit_width 4; WGSL kernels.
Rationale:  The GPU path must be meaningfully faster than the already-fast
            CPU Rayon path to justify the distribution complexity (R24).
            5 ms vs 80 ms is a 16× speedup over the CPU target.
Authority:  Perf lead
Ref:        FR-PERF-003, [[design/rust/risks-and-mitigations]] §R25
Tests:      None — tinyquant-gpu-wgpu crate not yet implemented (Phase 27).
Gap:        GAP-GPU-004 — GPU throughput benchmark requires a physical discrete GPU on a
            self-hosted runner; no such test can exist before Phase 27.
            See testing-gaps.md §GAP-GPU-004.
```

---

### FR-GPU-005 — GPU offload threshold gating

```
Gist:       The GPU backend shall report itself as unsuitable for batches
            below the minimum efficient batch size.
Type:       Function
Actor:      Library consumer
Function:   WgpuBackend::should_use_gpu(rows) shall return false when rows
            < WgpuBackend::BATCH_THRESHOLD. The threshold shall be
            configurable at compile time and documented.
Scale:      Correctness of should_use_gpu return value relative to the
            threshold constant.
Meter:      Unit test: call should_use_gpu for rows in
            {0, 1, BATCH_THRESHOLD - 1, BATCH_THRESHOLD,
            BATCH_THRESHOLD + 1, 10 000}; assert return values.
Must:       100% of calls return the correct boolean based on the threshold.
Plan:       Same.
Qualify:    GPU adapter present (is_available() == true).
Rationale:  Host↔device transfer overhead makes GPU dispatch inefficient
            for small batches. Without threshold gating, callers would see
            GPU path slower than CPU for the common interactive (single-
            vector) use case.
Ref:        [[design/rust/parallelism]] §GPU execution tier, FR-GPU-004
Tests:      [Crate]  tinyquant-gpu-wgpu/tests/batch_threshold.rs::should_use_gpu_returns_false_below_threshold
Gap:        None.
```

---

### FR-GPU-006 — PreparedCodec device upload is idempotent

```
Gist:       Calling prepare_for_device() on a PreparedCodec that is already
            prepared does not upload data again or return an error.
Type:       Function
Actor:      Library consumer
Function:   A second call to ComputeBackend::prepare_for_device(prepared)
            on a PreparedCodec whose gpu_state is already populated shall
            return Ok(()) without re-uploading buffers to the GPU.
Scale:      Fraction of repeated prepare_for_device() calls that return
            Ok(()) and produce no additional GPU buffer allocation.
Meter:      Call prepare_for_device() twice on the same PreparedCodec;
            assert Ok returned both times; assert GPU buffer count unchanged
            between calls (via a test instrumentation hook).
Must:       100%
Plan:       Same.
Qualify:    Same WgpuBackend instance and PreparedCodec between both calls.
Rationale:  Callers may call prepare_for_device() defensively before each
            batch without performance penalty; idempotency makes the API
            safe to use in retry loops.
Ref:        [[design/rust/risks-and-mitigations]] §R26
Tests:      [Crate]  tinyquant-gpu-wgpu/tests/parity_compress.rs::prepare_for_device_is_idempotent
Gap:        None.
```

---

### FR-GPU-007 — CUDA backend availability is feature-gated

```
Gist:       The tinyquant-gpu-cuda crate compiles to a stub (no link errors)
            when the `cuda` feature is disabled, even without a CUDA
            toolkit installed.
Type:       Constraint
Actor:      Library consumer on non-NVIDIA hardware
Function:   `cargo build -p tinyquant-gpu-cuda` (without `--features cuda`)
            shall succeed with no errors and produce an rlib where
            CudaBackend::is_available() always returns false.
Scale:      Build success (exit code 0) on a machine without CUDA toolkit.
Meter:      CI compile-check job: `cargo +1.87.0 check -p tinyquant-gpu-cuda`
            on ubuntu-22.04 without CUDA toolkit; assert exit 0.
Must:       Build succeeds; is_available() returns false.
Plan:       Same.
Qualify:    `cuda` feature NOT enabled; no CUDA toolkit in PATH.
Rationale:  If the CUDA crate fails to compile without a CUDA toolkit,
            it cannot be listed as an optional dependency for non-NVIDIA
            users. The stub pattern keeps the door open.
Ref:        [[design/rust/feature-flags]] §tinyquant-gpu-cuda,
            [[design/rust/risks-and-mitigations]] §R23
Tests:      None — tinyquant-gpu-cuda crate not yet implemented (Phase 28).
Gap:        GAP-GPU-007 — CUDA stub compile-check CI job and is_available() test cannot
            be written until Phase 28. See testing-gaps.md §GAP-GPU-007.
```

---

## See also

- [[requirements/performance|Performance Requirements]]
- [[design/rust/gpu-acceleration|GPU Acceleration Design]]
- [[design/rust/risks-and-mitigations|Risks and Mitigations]] §R23–R26
- [[design/rust/testing-strategy|Testing Strategy]] §GPU testing strategy
