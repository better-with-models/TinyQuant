---
title: Rust Port — Parallelism and Concurrency
tags:
  - design
  - rust
  - parallelism
  - rayon
  - threading
date-created: 2026-04-10
status: draft
category: design
---

# Rust Port — Parallelism and Concurrency

> [!info] Purpose
> Explain how the Rust port extracts data parallelism from batch
> operations without leaking concurrency concerns into the
> single-vector API.

## Principles

1. **Single-vector API is synchronous and single-threaded.**
   `Codec::compress(&self, vector, config, codebook)` never spawns a
   thread. Callers who need parallelism call the batch variant or
   drive their own thread pool.
2. **Batch API uses `rayon`.** `compress_batch` and
   `decompress_batch_into` use `rayon::slice::ParallelSlice` to split
   row ranges across the global thread pool.
3. **The core crate does not depend on `rayon`.** Parallelism is a
   leaf-crate concern. `tinyquant-core`'s batch methods take a
   `Parallelism` strategy parameter; the default is `Serial`.
   `tinyquant-io`, `tinyquant-bruteforce`, and `tinyquant-py` provide
   `Parallelism::Rayon` implementations.
4. **Thread pool is configured by the caller, not the library.**
   Callers set `RAYON_NUM_THREADS` or construct their own
   `rayon::ThreadPoolBuilder`. The library obeys.

## The `Parallelism` type

```rust
// tinyquant-core/src/codec/parallelism.rs
/// Execution strategy for batch operations.
#[derive(Clone, Copy, Debug, Default)]
pub enum Parallelism {
    /// Process rows sequentially. Zero overhead. Default.
    #[default]
    Serial,
    /// Callers supply a parallel-iteration callback.
    ///
    /// `RayonClosure` is the only variant today; the enum is
    /// open to future variants (e.g., Tokio blocking pool).
    Custom(fn(count: usize, body: &(dyn Fn(usize) + Sync + Send))),
}

impl Parallelism {
    #[inline]
    pub fn for_each_row<F>(self, count: usize, body: F)
    where
        F: Fn(usize) + Sync + Send,
    {
        match self {
            Self::Serial => {
                for i in 0..count {
                    body(i);
                }
            }
            Self::Custom(driver) => driver(count, &body),
        }
    }
}
```

In `tinyquant-io`:

```rust
pub fn rayon_parallelism() -> Parallelism {
    Parallelism::Custom(|count, body| {
        use rayon::prelude::*;
        (0..count).into_par_iter().for_each(|i| body(i));
    })
}
```

The core crate then calls:

```rust
impl Codec {
    pub fn compress_batch(
        &self,
        vectors: &[f32],
        rows: usize,
        cols: usize,
        config: &CodecConfig,
        codebook: &Codebook,
        parallelism: Parallelism,
    ) -> Result<Vec<CompressedVector>, CodecError> {
        self.validate_batch(vectors, rows, cols, config, codebook)?;

        // Preallocate slots; we write into them from parallel workers.
        let mut slots: Vec<core::mem::MaybeUninit<CompressedVector>> =
            (0..rows).map(|_| core::mem::MaybeUninit::uninit()).collect();
        let slot_ptrs = SyncPtr::new(slots.as_mut_ptr());

        parallelism.for_each_row(rows, |i| {
            let start = i * cols;
            let vector = &vectors[start..start + cols];
            let cv = self
                .compress(vector, config, codebook)
                .expect("per-row validation; batch validated above");
            // SAFETY: each index i is written exactly once by a single worker.
            unsafe {
                slot_ptrs.get().add(i).write(core::mem::MaybeUninit::new(cv));
            }
        });

        // SAFETY: all slots were initialized.
        let initialized = unsafe {
            core::mem::transmute::<
                Vec<core::mem::MaybeUninit<CompressedVector>>,
                Vec<CompressedVector>,
            >(slots)
        };
        Ok(initialized)
    }
}
```

The `SyncPtr` wrapper is a manual `unsafe impl Send + Sync for
SyncPtr<T>` newtype because raw pointers are `!Send` by default.
Each worker writes a unique index, and we do a happens-before
synchronization through rayon's join points, so the final `transmute`
is sound. The `unsafe` block is fully documented in-source.

An alternative that avoids `unsafe`:

```rust
let results: Vec<Result<CompressedVector, CodecError>> = match parallelism {
    Parallelism::Serial => (0..rows).map(|i| {
        let v = &vectors[i * cols..(i + 1) * cols];
        self.compress(v, config, codebook)
    }).collect(),
    Parallelism::Custom(_) => {
        // Use a rayon par_iter via a trampoline
        let mut out: Vec<_> = Vec::with_capacity(rows);
        out.resize_with(rows, || Err(CodecError::LengthMismatch { left: 0, right: 0 }));
        parallelism.for_each_row(rows, |i| {
            let v = &vectors[i * cols..(i + 1) * cols];
            // Need interior mutability here; see below.
            unreachable!("placeholder — real impl uses the unsafe version above");
        });
        out
    }
};
```

The `MaybeUninit` path is what we ship because it's faster and the
safety invariants are crystal clear. The alternative (`Result`
collection + filter + error propagation) costs an extra allocation
and a branch per row.

## Parallel GEMM for the rotation stage

At batch scale, the rotation step is:

```
rotated[n, d] = input_f64[n, d] @ R.T[d, d]
```

We call `faer::linalg::matmul` with `Parallelism::None` and let
rayon's outer thread pool split the row range. Two nested parallel
regions (faer's internal + rayon's outer) would oversubscribe; we
always set faer to `None` and drive parallelism at the codec level.

A future optimization (phase-16) switches to faer's internal
parallelism for batch sizes above a threshold where row-splitting is
suboptimal.

## Bringing your own rayon pool

TinyQuant never builds or owns a `rayon::ThreadPool`. Every parallel
entry point (`compress_batch`, `decompress_batch_into`,
`compress_batch_packed`) is driven by the `Parallelism::Custom`
closure, and `tinyquant-io::rayon_parallelism()` simply calls
`(0..n).into_par_iter()` on whatever pool is current at the call
site. This means downstream consumers that already have a
`rayon::ThreadPool` (better-router Rust agents, other rayon-using
services) should install their pool **once** and wrap batch calls in
a single `pool.install(...)` scope:

```rust
use rayon::ThreadPoolBuilder;
use tinyquant_io::rayon_parallelism;

let pool = ThreadPoolBuilder::new()
    .num_threads(num_cpus::get_physical())
    .thread_name(|i| format!("better-router-rayon-{i}"))
    .build()?;

let cvs = pool.install(|| {
    codec.compress_batch(
        &vectors, rows, cols, &cfg, &cb, rayon_parallelism(),
    )
})?;
```

This closes **R8** (rayon pool contention) from
[[design/rust/risks-and-mitigations|risks-and-mitigations.md]]: the
consumer's pool serves both its own workloads and TinyQuant's batch
paths, so nothing in the codec can fight with the host process over
CPU affinity or worker counts.

If no pool is installed, `rayon_parallelism()` falls back to the
global rayon pool (logical-core count), which is fine for ad-hoc
scripts, benchmarks, and the standalone CLI — but production
services that already embed `rayon` should not rely on that fallback.

## Thread pool configuration defaults

| Context | Default threads |
|---|---|
| `tinyquant-py` importing `tinyquant_rs` | `rayon::current_num_threads()` (defaults to logical cores) |
| `tinyquant-sys` consumers (C ABI) | Must call `tq_set_num_threads(n)` before first batch op |
| `tinyquant-bench` | Single thread by default; bench harness lets you override with `--threads N` |
| `tinyquant-bruteforce` | Inherits the current rayon pool |

The C ABI exposes a `tq_set_num_threads(usize)` function that installs
a rayon thread pool behind a `OnceCell` on first use. Calling it
after the pool is constructed is a no-op and logs a warning. The
function signature is:

```c
// tinyquant.h (generated)
TQ_API int32_t tq_set_num_threads(uintptr_t n);
```

## Global state audit

The Rust port has exactly these pieces of global mutable state:

1. The `RotationCache` (one instance per process, behind a
   `spin::Mutex`).
2. The dispatched ISA level (one `AtomicU8` per process, written once).
3. The rayon thread pool (one `OnceCell<rayon::ThreadPool>` in
   `tinyquant-sys` — created by `tq_set_num_threads`, otherwise the
   global default is used).

Nothing else. No logging globals, no metrics globals, no tracing
subscribers. The C ABI exposes a `tq_set_log_callback(fn)` function
for consumers that want structured logs; by default the library emits
nothing.

## Send + Sync audit

| Type | Send | Sync | Notes |
|---|---|---|---|
| `CodecConfig` | ✅ | ✅ | All fields are `Send + Sync` |
| `Codebook` | ✅ | ✅ | `Arc<[f32]>` is `Send + Sync` |
| `RotationMatrix` | ✅ | ✅ | `Arc<[f64]>` is `Send + Sync` |
| `CompressedVector` | ✅ | ✅ | All fields are `Arc` wrappers |
| `Codec` | ✅ | ✅ | ZST |
| `Corpus` | ✅ | ❌ | `&mut self` mutators; wrap in `Mutex` if cross-thread mutation needed |
| `CorpusEvent` | ✅ | ✅ | Same as above |
| `BruteForceBackend` | ✅ | ❌ | Interior store is `Vec`, not atomic |
| `PgvectorAdapter` | ✅ | ✅ | Connection factory is `Fn() + Send + Sync` |

## Lock-free read paths

The `RotationCache` optimizes the common case (cache hit on the first
entry) with a lock-free read:

```rust
impl RotationCache {
    pub fn get_or_build(&self, seed: u64, dim: u32) -> RotationMatrix {
        // Fast path: peek at the hot entry without locking.
        if let Some(m) = self.hot_entry.read_if_matches(seed, dim) {
            return m;
        }
        // Slow path: lock and check.
        let mut entries = self.entries.lock();
        if let Some(m) = entries.find(seed, dim) {
            self.hot_entry.publish(m.clone());
            return m;
        }
        let m = RotationMatrix::build(seed, dim);
        entries.push(m.clone());
        self.hot_entry.publish(m.clone());
        m
    }
}
```

`hot_entry` uses `arc_swap::ArcSwapOption<RotationMatrix>` for a
lock-free atomic pointer swap, so the hit path is a single atomic
load plus a seed/dim comparison. Benchmarks show ~35 ns per warm hit,
meeting the goal.

## Memory ordering choices

- `CACHED_ISA` uses `Relaxed` — the ISA level never changes after
  detection, so there's no synchronization to do.
- `hot_entry` uses `ArcSwapOption` which provides `Acquire`/`Release`
  semantics internally.
- The `RotationCache` mutex uses `spin::Mutex`, which has stronger
  guarantees than needed but is trivial to use.

No `unsafe impl Sync for X` anywhere outside the `SyncPtr` newtype in
the parallel batch path.

## GPU execution tier (Phase 27+)

GPU offload is a **third execution tier** above `Parallelism::Serial`
and `Parallelism::Custom(rayon)`. It is *not* an extension of the
`Parallelism` enum — it is a separate dispatch interface (`ComputeBackend`
trait) that lives in `tinyquant-gpu-wgpu` and `tinyquant-gpu-cuda`,
both of which depend on `tinyquant-core` but are never imported by it.

### Relationship between `Parallelism` and `ComputeBackend`

| Concern | `Parallelism` | `ComputeBackend` |
|---|---|---|
| Where defined | `tinyquant-core` | `tinyquant-gpu-wgpu` |
| Direction | CPU thread fan-out | Host→device transfer + kernel launch |
| Granularity | Per-row within a batch | Entire batch (whole tensor) |
| `no_std` compatible | Yes | No (GPU crates require `std`) |
| Default | `Serial` (CPU) | Not wired — opt-in by application |

### Threshold gating

GPU dispatch is only efficient above a minimum batch size (the
host↔device transfer overhead must be amortized). `WgpuBackend`
exposes:

```rust
impl WgpuBackend {
    /// Minimum row count above which GPU dispatch is faster than CPU.
    /// Below this threshold, callers should fall back to the CPU path.
    pub const BATCH_THRESHOLD: usize = 512;

    /// Associated function — no adapter check; use `is_available()` separately
    /// before dispatching to the GPU path.
    pub fn should_use_gpu(rows: usize) -> bool {
        rows >= Self::BATCH_THRESHOLD
    }
}
```

A typical dispatch pattern:

```rust
if backend.is_available() && WgpuBackend::should_use_gpu(rows) {
    backend.compress_batch(input, rows, cols, prepared)?
} else {
    codec.compress_batch_cpu(input, rows, cols, parallelism)?
}
```

The threshold is advisory; callers are free to override it (e.g., a
server with dedicated GPU capacity might lower it to 128).

### Fallback guarantee

`tinyquant-core` is always available as the CPU fallback. The GPU
crates declare `ComputeBackend::is_available() -> bool`; if this
returns `false` (no adapter found, driver unavailable, or the `cuda`
feature is disabled), the application falls back to `Parallelism::Custom`
on the CPU rayon pool. No GPU dependency ever bleeds into the core or
IO crates.

## Cancellation and interruption

The Rust port does not expose cancellation. Long-running batch
operations are expected to be driven by a caller that cares about
latency budgets and simply holds a shorter batch size. A future phase
can add `compress_batch_interruptible` behind a `tokio` feature if a
concrete need surfaces.

## Async runtime stance

`tinyquant-core` is sync. `tinyquant-io` is sync. `tinyquant-pgvector`
is sync (uses blocking `postgres` client). `tinyquant-py` exposes
sync methods. If a downstream consumer needs async, they wrap calls
in `tokio::task::spawn_blocking`.

Rationale: async would add dependency weight (`tokio`, `futures`),
obscure ownership, and buy nothing for a CPU-bound codec. Rejected.

## See also

- [[design/rust/simd-strategy|SIMD Strategy]]
- [[design/rust/memory-layout|Memory Layout]]
- [[design/rust/benchmark-harness|Benchmark Harness]]
- [[design/rust/ffi-and-bindings|FFI and Bindings]]
