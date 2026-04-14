# tinyquant-cli

Standalone command-line front-end for the
[TinyQuant](https://github.com/better-with-models/TinyQuant) CPU-only
vector-quantization codec.

The binary is published as `tinyquant` and exposes the codec, corpus,
verification, and benchmarking workflows needed to operate a TinyQuant
deployment without any host language bindings.

## Subcommands

| Command | Purpose |
| --- | --- |
| `tinyquant info` | Print version, build features, and environment info. |
| `tinyquant codec train` | Train a codebook on an input vector corpus. |
| `tinyquant codec compress` | Quantize a vector corpus against a trained codebook. |
| `tinyquant codec decompress` | Reconstruct vectors from a TinyQuant compressed stream. |
| `tinyquant verify` | Round-trip-verify a corpus and report MSE / fidelity. |

Run `tinyquant help <subcommand>` for arguments. Shell completions and
man pages are generated from `clap`'s derived parser.

## Install

```bash
# From the Rust release (recommended once Phase 22.D releases):
cargo install tinyquant-cli --locked

# Or from the TinyQuant repository directly:
git clone https://github.com/better-with-models/TinyQuant
cd TinyQuant/rust
cargo install --path crates/tinyquant-cli --locked
```

Pre-built binaries and container images are attached to each
[GitHub release](https://github.com/better-with-models/TinyQuant/releases).

## Cargo features

The crate exposes a narrow set of Cargo features tuned for operator
ergonomics. See the authoritative matrix in
[`docs/plans/rust/phase-22-pyo3-cabi-release.md`](../../../docs/plans/rust/phase-22-pyo3-cabi-release.md)
§CLI feature flag matrix.

| Feature | Default | Effect | Notes |
| --- | --- | --- | --- |
| `jemalloc` | on (non-MSVC) | Uses `jemallocator` as the global allocator | Gated by `cfg(not(target_env = "msvc"))`; disabled on musl targets. |
| `rayon` | on | Parallel batch compress/decompress honouring `--threads` | Disable for deterministic single-threaded runs. |
| `simd` | on | Portable SIMD kernels from `tinyquant-core` | Required to meet Phase 21 bench budgets on Tier-1 targets. |
| `mmap` | on | mmap-backed corpus reads in `corpus decompress` / `corpus search` | Falls back to `std::fs::read` when disabled. |
| `progress` | on | `indicatif` progress bars on long-running batches | `--no-progress`, `NO_COLOR=1`, `TERM=dumb` all suppress bars. |
| `tracing-json` | off | JSON-formatted logs via `tracing-subscriber` | Enabled in the release container image so `docker logs` emits structured output. |

## Compatibility

See [`COMPATIBILITY.md`](../../../COMPATIBILITY.md) for the
`(tinyquant_cpu, tinyquant_rs)` parity ledger that pins which Python
reference version a given CLI release round-trips against.

## License

Apache-2.0 — see the [workspace LICENSE](../../../LICENSE).
