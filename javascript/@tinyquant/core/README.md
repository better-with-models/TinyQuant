# @tinyquant/core

*CPU-only vector quantization codec for embedding storage compression.*

TinyQuant compresses high-dimensional embedding vectors to low-bit
representations while preserving cosine similarity rankings. This
package is the TypeScript / Node.js / Bun wrapper over the Rust
`tinyquant-core` crate, delivering the same codec / corpus / backend
shape available in the Python package (`tinyquant-cpu`) and on
crates.io (`tinyquant-core`).

## Install

```bash
npm install @tinyquant/core
# or
bun add @tinyquant/core
# or
pnpm add @tinyquant/core
```

Pin an exact version in production — npm rollback options are limited
(see `docs/plans/rust/phase-25-typescript-npm-package.md` §Rollback
plan):

```bash
npm install @tinyquant/core@0.1.0
```

## Supported platforms

Pre-built native binaries (via [napi-rs][]) ship for:

| Platform | Triple |
| :--- | :--- |
| Linux (x64, glibc) | `x86_64-unknown-linux-gnu` |
| Linux (arm64, glibc) | `aarch64-unknown-linux-gnu` |
| macOS (Intel) | `x86_64-apple-darwin` |
| macOS (Apple Silicon) | `aarch64-apple-darwin` |
| Windows (x64, MSVC) | `x86_64-pc-windows-msvc` |

Unsupported targets (Alpine musl, FreeBSD, Windows ARM64, browser)
raise a descriptive error at import time listing the detected
platform / arch and the supported combinations.

Runtime floors: **Node.js >= 20.10.0**, **Bun >= 1.1.0**. Both are
enforced via `engines` in `package.json`.

[napi-rs]: https://napi.rs/

## Quickstart

### Codec round-trip

```ts
import { Codec, CodecConfig } from "@tinyquant/core";

const config = new CodecConfig({ bitWidth: 4, dimension: 768, seed: 42 });
const codec = new Codec();

// Train a codebook on a representative sample.
const training = new Float32Array(1000 * 768);
crypto.getRandomValues(new Uint8Array(training.buffer));
const codebook = codec.buildCodebook(training, config);

// Compress + decompress one vector.
const vector = training.subarray(0, 768);
const compressed = codec.compress(vector, config, codebook);
const restored = codec.decompress(compressed, config, codebook);

// Expected: MSE < 1e-2 on random normal-ish input.
```

### Corpus + brute-force search

```ts
import {
  Codec, CodecConfig,
  Corpus, CompressionPolicy,
  BruteForceBackend,
} from "@tinyquant/core";

const config = new CodecConfig({ bitWidth: 4, dimension: 768, seed: 42 });
const codec = new Codec();
const codebook = codec.buildCodebook(trainingVectors, config);

const corpus = new Corpus(
  "my-vectors",
  config,
  codebook,
  CompressionPolicy.compress(),
);
for (let i = 0; i < trainingVectors.length / 768; i++) {
  corpus.insert(
    `vec-${i}`,
    trainingVectors.subarray(i * 768, (i + 1) * 768),
  );
}

const backend = new BruteForceBackend();
backend.ingest(corpus.decompressAll());
const results = backend.search(queryVector, 5);
for (const r of results) {
  console.log(`${r.vectorId}: ${r.score.toFixed(4)}`);
}
```

### CommonJS consumers

```js
const { version, Codec, CodecConfig } = require("@tinyquant/core");
console.log(version());
```

The package ships both ESM (`dist/*.js`) and CJS (`dist/*.cjs`)
entry points with matching `.d.ts` / `.d.cts` typings, so
TypeScript resolves the correct declaration file regardless of
the consumer's module system.

## Top-level exports

| Export | Shape | Notes |
| :--- | :--- | :--- |
| `version()` | `() => string` | Returns the semver of the installed native binding. |
| `Codec` | class | Codec value object with `buildCodebook`, `compress`, `decompress`, batch variants. |
| `CodecConfig` | class | Immutable config with deterministic `configHash` getter. |
| `Codebook` | class | Trained codebook, `toBytes()` / `fromBytes()` round-trip. |
| `CompressedVector` | class | Versioned binary serialization. |
| `RotationMatrix` | class | QR-derived orthogonal matrix; `apply()` rotates in place. |
| `Corpus` | class | Aggregate root with `insert`, `decompressAll`, `pendingEvents`. |
| `CompressionPolicy` | class (factories) | `compress()`, `passthrough()`, `fp16()` static factories. |
| `VectorEntry` | class | Record of a stored vector. Metadata is a no-op in 0.1.0 (see plan §Phase 25.3 declared deviations). |
| `BruteForceBackend` | class | Exhaustive cosine search. |
| `SearchResult` | class | `{ vectorId, score }`. |
| `TinyQuantError` | class | Structured error wrapper that preserves the Rust error class. |

Full API reference lives in the Obsidian vault at `docs/` in the
repo; the API surface is kebab-cased → camelCased from the Python
reference.

## Version alignment

The npm package version equals the Rust workspace version and the
Python fat-wheel version. Rust, Python, and TypeScript are always
released in lockstep. See the root `COMPATIBILITY.md` ledger.

## Rollback / deprecation

npm's rollback story is weaker than PyPI's: within 72 hours and
below 300 downloads, `npm unpublish @tinyquant/core@<ver>` is
allowed; otherwise use `npm deprecate`. See
`docs/plans/rust/phase-25-typescript-npm-package.md` §Rollback plan
for the full playbook.

## License

Apache-2.0 — see `LICENSE`.
