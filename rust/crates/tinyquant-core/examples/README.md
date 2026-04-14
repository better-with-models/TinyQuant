# rust/crates/tinyquant-core/examples

Developer binaries for generating deterministic binary fixtures from the Rust codec. Because Rust (ChaCha20) and Python (PCG64) use different RNG algorithms, byte-level parity against Python-generated files is not possible; these examples generate Rust-native fixtures used by the `codec_fixture_parity` and `rotation_fixture_parity` integration tests. Both binaries are gated on `feature = "std"` so they are excluded from the `no_std` build check.

## What lives here

### `dump_codec_fixture.rs`

Writes a full end-to-end codec fixture set. Usage:

```text
cargo run -p tinyquant-core --example dump_codec_fixture --features std -- \
    <input-seed> <codec-seed> <rows> <cols> \
    <training-f32-bin> <out-dir>
```

Outputs inside `<out-dir>/`: raw input vectors, quantized indices, residuals, and decompressed vectors — one file per bit-width (2, 4, 8).

### `dump_rotation_fixture.rs`

Writes a single rotation matrix as a raw little-endian `f64[dim × dim]` binary. Usage:

```text
cargo run -p tinyquant-core --example dump_rotation_fixture --features std -- \
    <seed> <dimension> <out-path>
```

## How this area fits the system

The output files from these examples are committed to `tests/fixtures/` and consumed by `tests/codec_fixture_parity.rs` and `tests/rotation_fixture_parity.rs`. When the codec changes in a way that intentionally alters output bytes, re-run the relevant example and replace the fixture files.

## Common edit paths

- **Fixture regeneration after an intentional codec change** — re-run the example, replace files in `../tests/fixtures/`, update the parity test if the file naming changes

## See also

- [Parent README](../README.md)
- [Local AGENTS.md](./AGENTS.md)
