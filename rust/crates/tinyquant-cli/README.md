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

## License

Apache-2.0 — see the [workspace LICENSE](../../../LICENSE).
