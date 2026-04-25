---
title: "Phase 25: TypeScript / Bun npm Package"
tags:
  - plans
  - rust
  - phase-25
  - typescript
  - javascript
  - npm
  - bun
  - node-api
  - napi-rs
date-created: 2026-04-13
status: complete
category: planning
---

# Phase 25: TypeScript / Bun npm Package

> [!info] Goal
> Ship `@tinyquant/core` on npm — a TypeScript wrapper over the Rust
> `tinyquant-core` crate, usable from Node.js ≥ 20 and Bun ≥ 1.1.
> The public API mirrors the Python fat-wheel wrapper delivered in
> [[plans/rust/phase-24-python-fat-wheel-official|Phase 24]] using
> idiomatic camelCase, so that TinyQuant ships with the same
> codec / corpus / backend shape across Python, TypeScript, and
> native Rust.
>
> Distribution uses the **bundled-binaries** strategy: one npm tarball
> carries all five platform `.node` files, and a tiny TypeScript
> loader (`src/_loader.ts`) picks the correct binary from
> `process.platform` + `process.arch` at import time. This mirrors the
> Phase 24 Python fat wheel rather than the per-platform optional-deps
> pattern that is napi-rs's default.

> [!note] Prerequisites
> - [[plans/rust/phase-22-pyo3-cabi-release|Phase 22]] complete:
>   `tinyquant-core` and `tinyquant-io` crates public, C ABI stable.
> - [[plans/rust/phase-24-python-fat-wheel-official|Phase 24]] complete
>   (or in flight): the fat wheel locks the cross-language API shape
>   that the TS wrapper must also satisfy.
> - Node.js ≥ 20 LTS is the floor; N-API version ≥ 9 is required by
>   napi-rs v3.
> - Bun ≥ 1.1 supports N-API well enough to load our `.node` files.
> - GitHub repo has **npm OIDC trusted publishing** enabled so
>   `npm publish --provenance` can sign releases without a long-lived
>   token.

## Why napi-rs over WebAssembly

TinyQuant has two plausible paths into the JavaScript ecosystem:
**N-API bindings** (`.node` files compiled per platform, loaded by
Node/Bun at runtime) and **WebAssembly** (portable `.wasm` loaded
by any JS runtime including browsers). For Phase 25 we pick N-API via
napi-rs. The trade-offs:

- **SIMD.** `tinyquant-core`'s Phase 20 kernels use AVX2, NEON, and
  `is_x86_feature_detected!` for runtime dispatch inside a native
  binary. WASM SIMD 128 is a portable subset that does not include
  `_mm256_*` (256-bit) intrinsics; wasm64 and relaxed-SIMD proposals
  are not yet stable across runtimes. N-API gives us full native SIMD
  on every supported platform with zero translation cost.
- **Zero-copy typed arrays.** Node and Bun back `Float32Array` with
  an `ArrayBuffer` whose memory is addressable from C. napi-rs's
  `TypedArray::as_ref()` gives us a `&[f32]` view over that buffer
  without copying. WASM requires memory to live inside the module's
  linear memory; every call either copies in/out or forces JS to
  allocate inside the wasm heap (`wasm-bindgen` does one or the
  other). For our 768-dim embedding workloads this is a 3 KB copy
  per vector in the WASM path versus zero bytes in the N-API path.
- **Concurrency.** There is no GIL in JavaScript. Node and Bun use
  Worker threads with separate V8/JSC isolates; `SharedArrayBuffer`
  is the cross-worker shared-memory path. napi-rs's `AsyncTask` +
  `tokio` or a thread pool can release the main JS thread while
  Rust work runs, analogous to PyO3's `allow_threads`. WASM can use
  threads with SharedArrayBuffer but the story is weaker: a single
  `.wasm` module instance is not safe to call concurrently without
  explicit locking.
- **Bundle size.** A stripped `.node` for a single platform is
  ~1.5 MB. A comparable `.wasm` is ~700 KB but carries a ~100 KB
  glue runtime. Bundled across five platforms the `.node` payload
  is larger (~8 MB total), but it's paid only once per host.
- **Debugging.** Rust stack frames show up in a `perf` / `dtrace`
  trace inside Node. WASM crashes surface as generic
  `WebAssembly.RuntimeError` with opaque offsets unless the consumer
  ships a source map.

WASM is **deferred, not ruled out**. A follow-up phase can publish
`@tinyquant/core-wasm` as a separate package targeting browser
consumers, sharing a TypeScript surface via the `_types.ts` module
introduced here. Phase 25 explicitly ships the Node/Bun-only
package.

## Distribution surface

| Artifact | Notes |
|---|---|
| `@tinyquant/core` | Main npm package; contains TypeScript wrapper + **bundled** `.node` binaries for all 5 supported platform/arch tuples. |
| GitHub Release `tinyquant-core-<version>.tgz` | Identical tarball attached to the GitHub Release for offline / enterprise installs. |
| Source tarball (auto) | npm publishes the package tarball; no separate source sdist. |

We deliberately do **not** publish the per-platform packages
(`@tinyquant/core-linux-x64-gnu`, etc.) that napi-rs's default
template would produce. The bundled-binaries strategy mirrors
[[plans/rust/phase-24-python-fat-wheel-official|Phase 24]], keeps
offline Docker builds single-tarball, and avoids the multi-package
publish race that has historically broken napi-rs releases when the
umbrella package resolves a newer version than the platform packages.

Rationale is identical to the fat-wheel argument in
[[research/python-rust-wrapper-distribution|Distribution Research]]:
install-time arch selection is replaced by a ~20-line runtime
loader in exchange for a single atomic artifact.

## tinyquant-js crate module map

`rust/crates/tinyquant-js/` is a new sibling to `tinyquant-py`. Cargo
workspace integration: add `"crates/tinyquant-js"` to the `members`
array in `rust/Cargo.toml`.

```toml
# rust/crates/tinyquant-js/Cargo.toml
[package]
name         = "tinyquant-js"
description  = "napi-rs N-API bindings for TinyQuant (published as @tinyquant/core)."
version.workspace      = true
edition.workspace      = true
rust-version.workspace = true
license.workspace      = true
repository.workspace   = true

[lib]
name       = "tinyquant_js"
crate-type = ["cdylib"]

[dependencies]
napi        = { version = "=3.0", default-features = false, features = ["napi9", "serde-json"] }
napi-derive = "=3.0"
tinyquant-core = { workspace = true }
tinyquant-io   = { workspace = true }
thiserror      = { workspace = true }

[build-dependencies]
napi-build = "=2.2"

[features]
default = ["simd"]
simd    = ["tinyquant-core/simd", "tinyquant-io/simd"]
```

Exact-version pins (`=3.0`) guard against the well-known churn between
napi-rs v2 and v3; minor-version bumps are reviewed manually. `napi9`
selects N-API level 9 which matches Node 20 LTS.

Module layout:

```
rust/crates/tinyquant-js/
├── Cargo.toml
├── build.rs                     ← one line: napi_build::setup();
├── package.json                 ← napi-rs tooling config (see below)
└── src/
    ├── lib.rs                   ← #[napi] root; re-exports sub-modules
    ├── version.rs               ← version() function + workspace-version constant
    ├── errors.rs                ← JsError mapping
    ├── buffers.rs               ← Float32Array / Uint8Array bridging helpers
    ├── codec.rs                 ← CodecConfig, Codebook, CompressedVector, RotationMatrix, compress/decompress
    ├── corpus.rs                ← Corpus, VectorEntry, CompressionPolicy, events
    └── backend.rs               ← BruteForceBackend, SearchResult
```

`rust/crates/tinyquant-js/package.json` (required by the `napi` CLI to
discover targets and name the output):

```json
{
  "name": "@tinyquant/core",
  "version": "0.1.0",
  "napi": {
    "name": "tinyquant",
    "triples": {
      "defaults": false,
      "additional": [
        "x86_64-unknown-linux-gnu",
        "aarch64-unknown-linux-gnu",
        "x86_64-apple-darwin",
        "aarch64-apple-darwin",
        "x86_64-pc-windows-msvc"
      ]
    }
  }
}
```

### Attribute macro conventions

Every Rust type exported across the boundary uses napi-rs's derive
macros. Conventions that apply consistently across all modules:

- `#[napi]` on structs that become JS classes.
- `#[napi(constructor)]` on a method named `new` to expose a
  JS `new X(...)` constructor.
- `#[napi(getter)]` on read-only properties (e.g. `config_hash`).
- `#[napi(factory)]` on `static fn new_*` patterns like
  `Codebook::train`.
- `#[napi(ts_type = "...")]` overrides where auto-inference of a TS
  type is wrong (notably for newtype wrappers around `String`).
- `#[napi(object)]` on plain-struct options bags such as
  `CodecConfigOpts`, which serialize as JS object literals rather
  than opaque classes.

### Typed-array bridging (`src/buffers.rs`)

Every FP32 / u8 boundary goes through this module so the zero-copy
policy is stated exactly once. The important rules:

- `Float32Array` → `&[f32]` is zero-copy **only** for sync methods.
  napi-rs exposes `TypedArray::as_ref()` which borrows the underlying
  `ArrayBuffer` for the duration of the JS call. The Rust side must
  not retain the reference past the end of the function.
- Async methods (those wrapped in `#[napi(ts_return_type = "Promise<...>")]`
  or `AsyncTask`) cannot hold the borrowed view across the `.await`
  boundary because V8 may move the buffer. The bridge copies the data
  into a `Vec<f32>` before spawning the work. This is explicit in the
  type signature — `compress(Float32Array)` is sync, `compressBatch`
  returns a `Promise` and pays the copy cost.
- Writing out: Rust returns `Vec<f32>` / `Vec<u8>`; napi-rs converts
  those into owned `Float32Array` / `Uint8Array` values with a single
  allocation, not a double-copy.

```rust
// src/buffers.rs
use napi::bindgen_prelude::{Float32Array, Uint8Array};
use napi::Result;

#[inline]
pub fn as_f32_slice(arr: &Float32Array) -> &[f32] {
    arr.as_ref()
}

#[inline]
pub fn as_u8_slice(arr: &Uint8Array) -> &[u8] {
    arr.as_ref()
}

pub fn to_f32_array(v: Vec<f32>) -> Float32Array {
    Float32Array::new(v)
}

pub fn to_u8_array(v: Vec<u8>) -> Uint8Array {
    Uint8Array::new(v)
}

pub fn require_dim(arr: &Float32Array, expected: usize, name: &'static str) -> Result<()> {
    if arr.len() != expected {
        return Err(napi::Error::from_reason(format!(
            "{name} length {} does not match dimension {expected}",
            arr.len()
        )));
    }
    Ok(())
}
```

### Error mapping (`src/errors.rs`)

Python's `DimensionMismatchError`, `ConfigMismatchError`,
`CodebookIncompatibleError`, and `DuplicateVectorError` become
JS errors of class `TinyQuantError` with a `.code` string matching
the Python class name. JavaScript has no inheritance-friendly
exception hierarchy we can meaningfully map onto per-class types,
so a single class with a discriminator preserves the information
without creating a zoo of `instanceof` checks.

```rust
// src/errors.rs
use napi::{Error, Status};

pub fn dim_mismatch(detail: impl Into<String>) -> Error {
    Error::new(Status::InvalidArg, format!("DimensionMismatchError: {}", detail.into()))
}

pub fn config_mismatch(detail: impl Into<String>) -> Error {
    Error::new(Status::InvalidArg, format!("ConfigMismatchError: {}", detail.into()))
}

pub fn codebook_incompatible(detail: impl Into<String>) -> Error {
    Error::new(Status::InvalidArg, format!("CodebookIncompatibleError: {}", detail.into()))
}

pub fn duplicate_vector(detail: impl Into<String>) -> Error {
    Error::new(Status::GenericFailure, format!("DuplicateVectorError: {}", detail.into()))
}
```

The TypeScript layer wraps these in a class:

```typescript
// src/_errors.ts
export class TinyQuantError extends Error {
  readonly code: string;
  constructor(code: string, message: string) {
    super(message);
    this.name = "TinyQuantError";
    this.code = code;
  }
}
```

A thin wrapper around every call site inspects the `Error` message
prefix and rethrows as `TinyQuantError` with a structured `code`.

### `src/version.rs`

```rust
use napi_derive::napi;

#[napi]
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
```

The value must equal the `@tinyquant/core` `package.json` version and
the Rust workspace version. A release pre-check enforces this (see
[[#Release versioning strategy]]).

## npm package tree

```
javascript/@tinyquant/core/
├── package.json                        ~1 KB
├── tsconfig.json                       ~0.5 KB
├── README.md                           ~4 KB
├── LICENSE                             ~12 KB (Apache-2.0 text)
├── dist/                                 (generated by `tsc`; published)
│   ├── index.js                        ~1 KB  (CJS shim)
│   ├── index.mjs                       ~1 KB  (ESM)
│   ├── index.d.ts                      ~12 KB
│   ├── index.d.cts                     ~12 KB (identical contents)
│   ├── codec.{js,mjs,d.ts,d.cts}
│   ├── corpus.{js,mjs,d.ts,d.cts}
│   ├── backend.{js,mjs,d.ts,d.cts}
│   ├── _loader.{js,mjs,d.ts}
│   ├── _errors.{js,mjs,d.ts}
│   └── _types.{js,mjs,d.ts}
├── binaries/                             (bundled native artifacts)
│   ├── linux-x64-gnu.node              ~1.6 MB
│   ├── linux-arm64-gnu.node            ~1.6 MB
│   ├── darwin-x64.node                 ~1.4 MB
│   ├── darwin-arm64.node               ~1.3 MB
│   └── win32-x64-msvc.node             ~1.8 MB
└── tests/                                (NOT published; .npmignore)
    ├── parity.test.ts
    ├── round-trip.test.ts
    ├── loader.test.ts
    └── fixtures/
        └── parity/
            ├── config_hashes.json
            ├── rotation_matrix_seed42_dim768.json
            ├── codebook_train_bw4.json
            ├── codebook_train_bw8.json
            ├── compressed_vector_round_trip.json
            └── brute_force_search.json
```

Published-package total size: ~8.5 MB uncompressed, ~4 MB gzipped.
Well under the npm soft-cap of ~100 MB where publish performance
degrades and comfortably within the Phase 24 50 MB budget.

### `package.json`

```json
{
  "name": "@tinyquant/core",
  "version": "0.1.0",
  "description": "CPU-only vector quantization codec for embedding storage compression.",
  "license": "Apache-2.0",
  "repository": {
    "type": "git",
    "url": "git+https://github.com/better-with-models/TinyQuant.git",
    "directory": "javascript/@tinyquant/core"
  },
  "engines": {
    "node": ">=20.10.0",
    "bun": ">=1.1.0"
  },
  "type": "module",
  "main": "./dist/index.js",
  "module": "./dist/index.mjs",
  "types": "./dist/index.d.ts",
  "exports": {
    ".": {
      "types": "./dist/index.d.ts",
      "import": "./dist/index.mjs",
      "require": "./dist/index.js"
    },
    "./codec": {
      "types": "./dist/codec.d.ts",
      "import": "./dist/codec.mjs",
      "require": "./dist/codec.js"
    },
    "./corpus": {
      "types": "./dist/corpus.d.ts",
      "import": "./dist/corpus.mjs",
      "require": "./dist/corpus.js"
    },
    "./backend": {
      "types": "./dist/backend.d.ts",
      "import": "./dist/backend.mjs",
      "require": "./dist/backend.js"
    },
    "./package.json": "./package.json"
  },
  "files": [
    "dist/",
    "binaries/",
    "README.md",
    "LICENSE"
  ],
  "keywords": [
    "vector",
    "quantization",
    "embedding",
    "compression",
    "tinyquant",
    "rust",
    "native"
  ],
  "scripts": {
    "build": "napi build --platform --release --output-dir binaries && tsc -p tsconfig.json",
    "test": "node --test tests/**/*.test.ts",
    "fixtures:generate": "python ../../../scripts/packaging/generate_js_parity_fixtures.py tests/fixtures"
  },
  "devDependencies": {
    "@napi-rs/cli": "3.0.0",
    "@types/node": "^20.10.0",
    "typescript": "^5.4.0",
    "detect-libc": "^2.0.3"
  }
}
```

### `tsconfig.json`

```json
{
  "compilerOptions": {
    "target": "ES2022",
    "module": "NodeNext",
    "moduleResolution": "NodeNext",
    "lib": ["ES2022"],
    "declaration": true,
    "declarationMap": true,
    "sourceMap": true,
    "strict": true,
    "esModuleInterop": true,
    "skipLibCheck": true,
    "forceConsistentCasingInFileNames": true,
    "outDir": "./dist",
    "rootDir": "./src",
    "types": ["node"]
  },
  "include": ["src/**/*.ts"],
  "exclude": ["node_modules", "dist", "tests"]
}
```

`module: NodeNext` is the right setting for libraries that want
both ESM and CJS consumers served off a single source tree. The
build produces both `.js` (CJS) and `.mjs` (ESM) outputs by running
`tsc` twice (once with `module: NodeNext` and once with
`module: CommonJS`) or by using a bundler; the CI step uses
`tsc` twice for simplicity.

## Runtime loader implementation

```typescript
// src/_loader.ts
import { createRequire } from "node:module";
import { fileURLToPath } from "node:url";
import path from "node:path";
import fs from "node:fs";

// Bun and Node ≥ 20.11 both set `import.meta.dirname`.
// Fallback covers Node 20.10.
const HERE =
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  (import.meta as any).dirname ??
  path.dirname(fileURLToPath(import.meta.url));

const req = createRequire(import.meta.url);

// Loading libc detection lazily so cold-start cost stays near zero on
// non-Linux platforms. `detect-libc` synchronously probes
// `getconf GNU_LIBC_VERSION` or `/lib/ld-musl-*` as appropriate.
function linuxVariant(): "gnu" | "musl" {
  try {
    // We deliberately use the library (battle-tested across Alpine/
    // Debian/RHEL/Amazon Linux) rather than hand-rolling a probe of
    // `/etc/alpine-release`. The library also handles containerized
    // environments where `/etc/os-release` lies about the libc.
    const detect = req("detect-libc") as { familySync: () => string };
    return detect.familySync() === "musl" ? "musl" : "gnu";
  } catch {
    // If detect-libc is missing (shouldn't happen — it's a direct
    // dep) default to gnu, which matches the most common Linux host.
    return "gnu";
  }
}

export function binaryKey(): string {
  const { platform, arch } = process;

  if (platform === "linux") {
    const libc = linuxVariant();
    if (arch === "x64") return `linux-x64-${libc}`;
    if (arch === "arm64") return `linux-arm64-${libc}`;
  } else if (platform === "darwin") {
    if (arch === "x64") return "darwin-x64";
    if (arch === "arm64") return "darwin-arm64";
  } else if (platform === "win32") {
    if (arch === "x64") return "win32-x64-msvc";
  }

  throw new Error(
    `@tinyquant/core: no pre-built binary for ${platform}/${arch}. ` +
      `Supported combinations: linux/x64, linux/arm64, darwin/x64, ` +
      `darwin/arm64, win32/x64. Please open an issue at ` +
      `https://github.com/better-with-models/TinyQuant/issues.`,
  );
}

type NativeBinding = {
  version: () => string;
  // Codec
  CodecConfig: unknown;
  Codebook: unknown;
  CompressedVector: unknown;
  RotationMatrix: unknown;
  compress: unknown;
  decompress: unknown;
  // Corpus
  Corpus: unknown;
  VectorEntry: unknown;
  CompressionPolicy: unknown;
  // Backend
  BruteForceBackend: unknown;
  SearchResult: unknown;
};

function loadNative(): NativeBinding {
  const key = binaryKey();
  // Keep path.join so Windows backslashes are inserted correctly —
  // `require()` on Windows accepts either separator but
  // `path.join` produces the platform-native form, which shows up
  // cleanly in stack traces.
  const candidate = path.join(HERE, "..", "binaries", `${key}.node`);

  if (!fs.existsSync(candidate)) {
    throw new Error(
      `@tinyquant/core: expected bundled binary at ${candidate} ` +
        `but file is missing. The package tarball may have been ` +
        `truncated; try reinstalling.`,
    );
  }

  try {
    return req(candidate) as NativeBinding;
  } catch (err) {
    const detail = err instanceof Error ? err.message : String(err);
    throw new Error(
      `@tinyquant/core: failed to load native binary at ${candidate}: ${detail}`,
    );
  }
}

export const native = loadNative();
```

Notes:

- `musl` support is added to the binary key even though Phase 25 does
  not ship a musl build yet. The loader returns a clear error on
  `linux-x64-musl` rather than silently loading a glibc binary that
  would fail with a cryptic `ld-linux` error at the first syscall. A
  follow-up phase can add Alpine builds without changing the loader.
- We use `detect-libc` (small, single dependency, widely depended
  on by `sharp` and `@napi-rs/image`) rather than probing
  `/etc/alpine-release` directly. Probing files is fragile in
  minimal containers; `detect-libc` has been hardened against them.
- Windows pathing: `path.join(HERE, "..", "binaries", …)` yields
  `C:\Users\…\binaries\win32-x64-msvc.node`. Node's `require`
  accepts backslashes, but `require` with an absolute posix-style
  path on Windows has historically failed — always use `path.join`.

## API surface (mirrors Python fat wheel)

The API is a direct camelCase mapping of the
`tinyquant_cpu` surface described in Phase 24 §API parity. Every
Python name below appears in [[#API parity specification]].

### `src/codec.ts`

```typescript
// src/codec.ts
import { native } from "./_loader.js";

export type ConfigHash = string; // 64-hex-char SHA-256 digest

export interface CodecConfigOpts {
  bitWidth: 2 | 4 | 8;
  seed: bigint; // u64 — JS bigint to fit full range
  dimension: number; // u32
  residualEnabled?: boolean; // default true
}

export declare class CodecConfig {
  constructor(opts: CodecConfigOpts);
  readonly bitWidth: number;
  readonly seed: bigint;
  readonly dimension: number;
  readonly residualEnabled: boolean;
  readonly numCodebookEntries: number;
  readonly configHash: ConfigHash;
}

export declare class Codebook {
  static train(vectors: Float32Array, config: CodecConfig): Codebook;
  readonly bitWidth: number;
  readonly numEntries: number;
  readonly entries: Float32Array;
  quantize(values: Float32Array): Uint8Array;
  dequantize(indices: Uint8Array): Float32Array;
}

export declare class CompressedVector {
  static fromBytes(data: Uint8Array): CompressedVector;
  toBytes(): Uint8Array;
  readonly configHash: ConfigHash;
  readonly dimension: number;
  readonly bitWidth: number;
  readonly hasResidual: boolean;
  readonly sizeBytes: number;
  readonly indices: Uint8Array;
}

export declare class RotationMatrix {
  static fromConfig(config: CodecConfig): RotationMatrix;
  readonly seed: bigint;
  readonly dimension: number;
  apply(vector: Float32Array): Float32Array;
  applyInverse(rotated: Float32Array): Float32Array;
  verifyOrthogonality(tol?: number): boolean;
}

export declare class Codec {
  constructor();
  compress(
    vector: Float32Array,
    config: CodecConfig,
    codebook: Codebook,
  ): CompressedVector;
  decompress(
    compressed: CompressedVector,
    config: CodecConfig,
    codebook: Codebook,
  ): Float32Array;
  compressBatch(
    vectors: Float32Array[],
    config: CodecConfig,
    codebook: Codebook,
  ): Promise<CompressedVector[]>;
  decompressBatch(
    compressed: CompressedVector[],
    config: CodecConfig,
    codebook: Codebook,
  ): Promise<Float32Array[]>;
  buildCodebook(
    trainingVectors: Float32Array,
    config: CodecConfig,
  ): Codebook;
  buildRotation(config: CodecConfig): RotationMatrix;
}

// Module-level convenience fns (matches Python compress/decompress shortcuts)
export declare function compress(
  vector: Float32Array,
  config: CodecConfig,
  codebook: Codebook,
): CompressedVector;
export declare function decompress(
  compressed: CompressedVector,
  config: CodecConfig,
  codebook: Codebook,
): Float32Array;

// Re-bind runtime objects from the native binding
const n = native as unknown as {
  CodecConfig: typeof CodecConfig;
  Codebook: typeof Codebook;
  CompressedVector: typeof CompressedVector;
  RotationMatrix: typeof RotationMatrix;
  Codec: typeof Codec;
  compress: typeof compress;
  decompress: typeof decompress;
};

export const CodecConfig: typeof CodecConfig = n.CodecConfig;
export const Codebook: typeof Codebook = n.Codebook;
export const CompressedVector: typeof CompressedVector = n.CompressedVector;
export const RotationMatrix: typeof RotationMatrix = n.RotationMatrix;
export const Codec: typeof Codec = n.Codec;
export const compress = n.compress;
export const decompress = n.decompress;
```

### `src/corpus.ts`

```typescript
import { native } from "./_loader.js";
import type { CodecConfig, Codebook, CompressedVector, ConfigHash } from "./codec.js";

export type CompressionPolicyKind = "compress" | "passthrough" | "fp16";

export declare class CompressionPolicy {
  static readonly COMPRESS: CompressionPolicy;
  static readonly PASSTHROUGH: CompressionPolicy;
  static readonly FP16: CompressionPolicy;
  readonly kind: CompressionPolicyKind;
  requiresCodec(): boolean;
  storageDtype(): "uint8" | "float16" | "float32";
}

export declare class VectorEntry {
  readonly vectorId: string;
  readonly compressed: CompressedVector;
  readonly insertedAt: Date;
  readonly metadata: Record<string, unknown> | null;
  readonly configHash: ConfigHash;
  readonly dimension: number;
  readonly hasResidual: boolean;
}

export interface CorpusOpts {
  corpusId: string;
  codecConfig: CodecConfig;
  codebook: Codebook;
  compressionPolicy: CompressionPolicy;
  metadata?: Record<string, unknown>;
}

export interface CorpusEvent {
  readonly type:
    | "CorpusCreated"
    | "VectorsInserted"
    | "CorpusDecompressed"
    | "CompressionPolicyViolationDetected";
  readonly corpusId: string;
  readonly timestamp: Date;
  readonly [key: string]: unknown;
}

export declare class Corpus {
  constructor(opts: CorpusOpts);
  readonly corpusId: string;
  readonly codecConfig: CodecConfig;
  readonly codebook: Codebook;
  readonly compressionPolicy: CompressionPolicy;
  readonly metadata: Record<string, unknown>;
  readonly vectorCount: number;
  readonly isEmpty: boolean;
  readonly vectorIds: ReadonlySet<string>;

  insert(
    vectorId: string,
    vector: Float32Array,
    metadata?: Record<string, unknown>,
  ): VectorEntry;
  insertBatch(
    vectors: Record<string, Float32Array>,
    metadata?: Record<string, Record<string, unknown>>,
  ): VectorEntry[];
  get(vectorId: string): VectorEntry;
  contains(vectorId: string): boolean;
  decompress(vectorId: string): Float32Array;
  decompressAll(): Record<string, Float32Array>;
  // `remove` returns `true` if the vector was removed, `false` if it
  // was not present — mirrors Python's
  // `dict.pop(vector_id, None) is not None` semantics used by the
  // reference oracle. Reconciled with implementation in the
  // Phase 25.3 spec-review fix pass (see `src/corpus.ts:328`).
  remove(vectorId: string): boolean;

  pendingEvents(): CorpusEvent[];
}

const n = native as unknown as {
  CompressionPolicy: typeof CompressionPolicy;
  VectorEntry: typeof VectorEntry;
  Corpus: typeof Corpus;
};
export const CompressionPolicy: typeof CompressionPolicy = n.CompressionPolicy;
export const VectorEntry: typeof VectorEntry = n.VectorEntry;
export const Corpus: typeof Corpus = n.Corpus;
```

### `src/backend.ts`

```typescript
import { native } from "./_loader.js";

export declare class SearchResult {
  readonly vectorId: string;
  readonly score: number;
}

export declare class BruteForceBackend {
  constructor();
  readonly count: number;
  ingest(vectors: Record<string, Float32Array>): void;
  search(query: Float32Array, topK: number): SearchResult[];
  remove(vectorIds: readonly string[]): void;
  clear(): void;
}

const n = native as unknown as {
  SearchResult: typeof SearchResult;
  BruteForceBackend: typeof BruteForceBackend;
};
export const SearchResult: typeof SearchResult = n.SearchResult;
export const BruteForceBackend: typeof BruteForceBackend = n.BruteForceBackend;
```

### `src/index.ts`

```typescript
export * from "./codec.js";
export * from "./corpus.js";
export * from "./backend.js";
export { TinyQuantError } from "./_errors.js";
export { version } from "./_loader.js"; // re-expose native.version() if desired
```

## API parity specification

Every public name in the Python fat wheel has a corresponding TS name.
The table is the single source of truth the TS implementation is
validated against in CI.

### `tinyquant_cpu.codec` → `@tinyquant/core/codec`

| Python | TypeScript | Notes |
|---|---|---|
| `CodecConfig(bit_width, seed, dimension, residual_enabled=True)` | `new CodecConfig({ bitWidth, seed, dimension, residualEnabled })` | Options-object constructor |
| `CodecConfig.bit_width` (field) | `CodecConfig.bitWidth` (readonly getter) | |
| `CodecConfig.seed` | `CodecConfig.seed` (bigint) | u64 |
| `CodecConfig.dimension` | `CodecConfig.dimension` (number) | u32 |
| `CodecConfig.residual_enabled` | `CodecConfig.residualEnabled` | |
| `CodecConfig.num_codebook_entries` (property) | `CodecConfig.numCodebookEntries` (readonly getter) | |
| `CodecConfig.config_hash` (property) | `CodecConfig.configHash` (readonly getter) | 64-hex SHA-256 |
| `Codebook(entries, bit_width)` | *Not directly constructable from JS* | Only `train` + native-side private ctor |
| `Codebook.train(cls, vectors, config)` | `static Codebook.train(vectors, config)` | |
| `Codebook.entries` (NDArray[f32]) | `Codebook.entries` (Float32Array) | |
| `Codebook.bit_width` | `Codebook.bitWidth` | |
| `Codebook.num_entries` | `Codebook.numEntries` | |
| `Codebook.quantize(values)` | `Codebook.quantize(values)` | |
| `Codebook.dequantize(indices)` | `Codebook.dequantize(indices)` | |
| `CompressedVector.indices` | `CompressedVector.indices` | |
| `CompressedVector.residual` (bytes \| None) | *Not exposed directly* | Round-tripped through `toBytes`/`fromBytes` |
| `CompressedVector.config_hash` | `CompressedVector.configHash` | |
| `CompressedVector.dimension` | `CompressedVector.dimension` | |
| `CompressedVector.bit_width` | `CompressedVector.bitWidth` | |
| `CompressedVector.has_residual` | `CompressedVector.hasResidual` | |
| `CompressedVector.size_bytes` | `CompressedVector.sizeBytes` | |
| `CompressedVector.to_bytes()` | `CompressedVector.toBytes()` | |
| `CompressedVector.from_bytes(cls, data)` | `static CompressedVector.fromBytes(data)` | |
| `RotationMatrix.from_config(cls, config)` | `static RotationMatrix.fromConfig(config)` | |
| `RotationMatrix.matrix` (NDArray[f64]) | *Not exposed* | Internal implementation detail |
| `RotationMatrix.seed` | `RotationMatrix.seed` (bigint) | |
| `RotationMatrix.dimension` | `RotationMatrix.dimension` | |
| `RotationMatrix.apply(v)` | `RotationMatrix.apply(v)` | |
| `RotationMatrix.apply_inverse(v)` | `RotationMatrix.applyInverse(v)` | |
| `RotationMatrix.verify_orthogonality(tol=1e-6)` | `RotationMatrix.verifyOrthogonality(tol?)` | |
| `Codec().compress(v, cfg, cb)` | `new Codec().compress(v, cfg, cb)` | |
| `Codec().decompress(cv, cfg, cb)` | `Codec.decompress` | |
| `Codec().compress_batch(vecs, cfg, cb)` | `Codec.compressBatch(vecs, cfg, cb)` → `Promise<CompressedVector[]>` | async for GIL-release parity |
| `Codec().decompress_batch(cvs, cfg, cb)` | `Codec.decompressBatch` → `Promise<Float32Array[]>` | |
| `Codec().build_codebook(tv, cfg)` | `Codec.buildCodebook(tv, cfg)` | |
| `Codec().build_rotation(cfg)` | `Codec.buildRotation(cfg)` | |
| `codec.compress(...)` (module fn) | `import { compress } from "@tinyquant/core/codec"` | |
| `codec.decompress(...)` | `import { decompress } from "@tinyquant/core/codec"` | |
| `DimensionMismatchError` | `TinyQuantError` with `code === "DimensionMismatchError"` | |
| `ConfigMismatchError` | `TinyQuantError` with `code === "ConfigMismatchError"` | |
| `CodebookIncompatibleError` | `TinyQuantError` with `code === "CodebookIncompatibleError"` | |
| `DuplicateVectorError` | `TinyQuantError` with `code === "DuplicateVectorError"` | |

### `tinyquant_cpu.corpus` → `@tinyquant/core/corpus`

| Python | TypeScript |
|---|---|
| `CompressionPolicy.COMPRESS` | `CompressionPolicy.COMPRESS` |
| `CompressionPolicy.PASSTHROUGH` | `CompressionPolicy.PASSTHROUGH` |
| `CompressionPolicy.FP16` | `CompressionPolicy.FP16` |
| `.requires_codec()` | `.requiresCodec()` |
| `.storage_dtype()` | `.storageDtype()` → `"uint8" \| "float16" \| "float32"` |
| `Corpus(corpus_id, codec_config, codebook, compression_policy, metadata=None)` | `new Corpus({ corpusId, codecConfig, codebook, compressionPolicy, metadata? })` |
| `.corpus_id` | `.corpusId` |
| `.codec_config` | `.codecConfig` |
| `.codebook` | `.codebook` |
| `.compression_policy` | `.compressionPolicy` |
| `.metadata` | `.metadata` |
| `.vector_count` | `.vectorCount` |
| `.is_empty` | `.isEmpty` |
| `.vector_ids` → `frozenset[str]` | `.vectorIds` → `ReadonlySet<string>` |
| `.insert(vector_id, vector, metadata=None)` | `.insert(vectorId, vector, metadata?)` |
| `.insert_batch(vectors, metadata=None)` | `.insertBatch(vectors, metadata?)` |
| `.get(vector_id)` | `.get(vectorId)` |
| `.contains(vector_id)` | `.contains(vectorId)` |
| `.decompress(vector_id)` | `.decompress(vectorId)` |
| `.decompress_all()` | `.decompressAll()` → `Record<string, Float32Array>` |
| `.remove(vector_id)` | `.remove(vectorId)` |
| `.pending_events()` | `.pendingEvents()` |
| `VectorEntry(vector_id, compressed, inserted_at, metadata=None)` | `VectorEntry` (native ctor) |
| `.vector_id / .compressed / .inserted_at / .metadata` | `.vectorId / .compressed / .insertedAt (Date) / .metadata` |
| `.config_hash / .dimension / .has_residual` | `.configHash / .dimension / .hasResidual` |
| `CorpusCreated`, `VectorsInserted`, `CorpusDecompressed`, `CompressionPolicyViolationDetected` | Union `CorpusEvent` with `type` discriminator |

### `tinyquant_cpu.backend` → `@tinyquant/core/backend`

| Python | TypeScript |
|---|---|
| `SearchResult(vector_id, score)` | `SearchResult` (native ctor) |
| `.vector_id` / `.score` | `.vectorId` / `.score` |
| `BruteForceBackend()` | `new BruteForceBackend()` |
| `.count` | `.count` |
| `.ingest(vectors)` | `.ingest(vectors)` |
| `.search(query, top_k)` | `.search(query, topK)` |
| `.remove(vector_ids)` | `.remove(vectorIds)` |
| `.clear()` | `.clear()` |
| `SearchBackend` Protocol | *Structural — TS users satisfy `BruteForceBackend`'s shape* |

## Type surface — generated `.d.ts` (representative slice)

`napi build --dts dist/index.d.ts` emits type declarations directly
from the `#[napi]` annotations. A representative excerpt of what it
produces (before the TS wrapper files are layered on top):

```typescript
/* auto-generated by @napi-rs/cli; do not edit */
/* eslint-disable */

export function version(): string;

export interface CodecConfigOpts {
  bitWidth: number;
  seed: bigint;
  dimension: number;
  residualEnabled?: boolean;
}

export declare class CodecConfig {
  constructor(opts: CodecConfigOpts);
  get bitWidth(): number;
  get seed(): bigint;
  get dimension(): number;
  get residualEnabled(): boolean;
  get numCodebookEntries(): number;
  get configHash(): string;
}

export declare class Codebook {
  static train(vectors: Float32Array, config: CodecConfig): Codebook;
  get bitWidth(): number;
  get numEntries(): number;
  get entries(): Float32Array;
  quantize(values: Float32Array): Uint8Array;
  dequantize(indices: Uint8Array): Float32Array;
}

export declare class CompressedVector {
  static fromBytes(data: Uint8Array): CompressedVector;
  toBytes(): Uint8Array;
  get configHash(): string;
  get dimension(): number;
  get bitWidth(): number;
  get hasResidual(): boolean;
  get sizeBytes(): number;
  get indices(): Uint8Array;
}
```

Notes on the generation:

- `u64` → `bigint`. `u32` → `number`. napi-rs validates range at
  the boundary.
- `get foo(): T` is the napi-rs idiom for `#[napi(getter)]`.
- `static` methods on classes match `#[napi(factory)]` attributes.
- `Float32Array` / `Uint8Array` are the zero-copy typed-array types.
- The options-object pattern (`CodecConfigOpts`) is surfaced via
  `#[napi(object)]` on a plain Rust struct.

The TS wrapper files (`src/codec.ts` etc.) re-export the generated
types with nicer JSDoc and explicit names, so that users importing
from `@tinyquant/core` see hand-polished docstrings rather than the
auto-gen output.

## Parity test fixtures

Cross-language parity is checked by loading JSON fixtures generated
from the Python reference implementation. This keeps the TS test
suite hermetic — it does not shell out to Python at test time, it
does not require Python to be installed, and it does not use
`child_process.exec`. A developer regenerates fixtures manually
when the Python implementation changes, and the resulting JSON is
committed.

Generator script: `scripts/packaging/generate_js_parity_fixtures.py`.

```python
"""Regenerate JS parity fixtures. Run manually; commit the output.

Usage:
    python scripts/packaging/generate_js_parity_fixtures.py \
        javascript/@tinyquant/core/tests/fixtures
"""
from __future__ import annotations
import json
import sys
from pathlib import Path
import numpy as np
import tinyquant_cpu as tq   # Phase 24 fat wheel (identical to tinyquant_cpu.codec)

OUT = Path(sys.argv[1]) / "parity"
OUT.mkdir(parents=True, exist_ok=True)
SEED = 42
DIM = 768

# 1. config_hashes.json
cases = [
    {"bit_width": bw, "seed": s, "dimension": d, "residual_enabled": r}
    for bw in (2, 4, 8)
    for s in (0, 1, 42, 123, 999)
    for d in (64, 384, 768, 1536)
    for r in (True, False)
]
out = []
for c in cases:
    cfg = tq.codec.CodecConfig(**c)
    out.append({**c, "config_hash": cfg.config_hash})
(OUT / "config_hashes.json").write_text(json.dumps(out, indent=2))

# 2. rotation_matrix_seed42_dim768.json
cfg = tq.codec.CodecConfig(bit_width=4, seed=SEED, dimension=DIM)
rot = tq.codec.RotationMatrix.from_config(cfg)
rng = np.random.default_rng(0)
vec = rng.standard_normal(DIM).astype(np.float32)
(OUT / "rotation_matrix_seed42_dim768.json").write_text(
    json.dumps({
        "seed": SEED, "dimension": DIM,
        "input_vector": vec.tolist(),
        "rotated": rot.apply(vec).tolist(),
    }, indent=2)
)

# 3. codebook_train_bw4.json
train = np.random.default_rng(7).standard_normal((10_000, DIM)).astype(np.float32)
cb = tq.codec.Codebook.train(train, cfg)
(OUT / "codebook_train_bw4.json").write_text(
    json.dumps({
        "bit_width": 4, "dimension": DIM, "seed": SEED,
        "training_shape": list(train.shape),
        "training_flat_first1k": train.flatten()[:1000].tolist(),
        "entries": cb.entries.tolist(),
    }, indent=2)
)

# 4. codebook_train_bw8.json — identical but bw=8
# 5. compressed_vector_round_trip.json — compress 50 vectors and dump .to_bytes() as base64
# 6. brute_force_search.json — ingest 256 vectors, run 5 queries, dump ranked ids+scores
```

Fixture files and what each checks:

- `config_hashes.json` — 120 triples; TS computes `configHash` and
  asserts byte-equality of the hex string. This is the single
  strongest parity gate in the suite.
- `rotation_matrix_seed42_dim768.json` — loads the reference input
  vector, runs `RotationMatrix.fromConfig(...).apply(...)`, and
  asserts `allclose(rotated, expected, atol=1e-5)`. Catches drift
  in the QR-decomposition sign-correction path.
- `codebook_train_bw4.json` / `...bw8.json` — loads the training
  batch, runs `Codebook.train(...)` in TS, asserts `entries` match
  exactly (entries are deterministic under the documented training
  algorithm).
- `compressed_vector_round_trip.json` — 50 vectors compressed on
  Python; TS calls `CompressedVector.fromBytes()` on each, then
  `decompress()`, and asserts MSE < 1e-2 against the original
  Python decompression. Also asserts `toBytes()` from the TS
  recompression equals the recorded Python bytes.
- `brute_force_search.json` — 256-vector corpus, 5 queries,
  top-10; TS asserts the same ranked IDs and scores within 1e-6.

Regeneration policy: fixtures are regenerated only when the
reference Python implementation changes, and the regeneration PR
is required to describe the semantic change. CI **does not**
regenerate fixtures; it only consumes them.

## CI workflow — `js-ci.yml`

```yaml
# .github/workflows/js-ci.yml
name: js-ci
on:
  push:
    branches: [main]
    paths:
      - "rust/crates/tinyquant-js/**"
      - "rust/crates/tinyquant-core/**"
      - "rust/crates/tinyquant-io/**"
      - "javascript/**"
      - ".github/workflows/js-ci.yml"
  pull_request:
    paths:
      - "rust/crates/tinyquant-js/**"
      - "javascript/**"

concurrency:
  group: js-ci-${{ github.ref }}
  cancel-in-progress: true

jobs:
  build-native:
    name: build .node (${{ matrix.triple }})
    strategy:
      fail-fast: false
      matrix:
        include:
          - { triple: x86_64-unknown-linux-gnu,   runner: ubuntu-22.04,     key: linux-x64-gnu }
          - { triple: aarch64-unknown-linux-gnu,  runner: ubuntu-22.04-arm, key: linux-arm64-gnu }
          - { triple: x86_64-apple-darwin,        runner: macos-13,         key: darwin-x64 }
          - { triple: aarch64-apple-darwin,       runner: macos-14,         key: darwin-arm64 }
          - { triple: x86_64-pc-windows-msvc,     runner: windows-2022,     key: win32-x64-msvc }
    runs-on: ${{ matrix.runner }}
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with: { targets: ${{ matrix.triple }} }
      - uses: actions/setup-node@v4
        with: { node-version: "20.x" }
      - name: Install @napi-rs/cli
        run: npm install -g @napi-rs/cli@3.0.0
      - name: Build native
        working-directory: rust/crates/tinyquant-js
        run: napi build --release --target ${{ matrix.triple }} --output-dir ../../../javascript/@tinyquant/core/binaries
        env:
          RUSTFLAGS: "-C strip=symbols"
      - name: Rename native artifact
        shell: bash
        working-directory: javascript/@tinyquant/core/binaries
        run: |
          # napi-rs outputs tinyquant.<triple>.node — rename to <key>.node
          src=$(ls tinyquant.*.node)
          mv "$src" "${{ matrix.key }}.node"
      - uses: actions/upload-artifact@v4
        with:
          name: native-${{ matrix.key }}
          path: javascript/@tinyquant/core/binaries/${{ matrix.key }}.node

  assemble-tarball:
    name: assemble npm tarball
    needs: build-native
    runs-on: ubuntu-22.04
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with: { node-version: "20.x" }
      - uses: actions/download-artifact@v4
        with:
          path: javascript/@tinyquant/core/binaries
          pattern: native-*
          merge-multiple: true
      - name: TypeScript compile
        working-directory: javascript/@tinyquant/core
        run: |
          npm ci
          npx tsc -p tsconfig.json
          # Emit CJS variant
          npx tsc -p tsconfig.json --module CommonJS --outDir dist --declaration false
      - name: npm pack
        working-directory: javascript/@tinyquant/core
        run: npm pack --pack-destination ../../../dist-npm
      - uses: actions/upload-artifact@v4
        with: { name: npm-tarball, path: dist-npm/*.tgz }

  install-test:
    name: install+test (${{ matrix.runner }}, ${{ matrix.runtime }})
    needs: assemble-tarball
    strategy:
      fail-fast: false
      matrix:
        runner: [ubuntu-22.04, ubuntu-22.04-arm, macos-13, macos-14, windows-2022]
        runtime: [node-20, node-22, bun-1.1]
    runs-on: ${{ matrix.runner }}
    steps:
      - uses: actions/checkout@v4
      - uses: actions/download-artifact@v4
        with: { name: npm-tarball, path: ./tarball }
      - name: Setup Node
        if: startsWith(matrix.runtime, 'node-')
        uses: actions/setup-node@v4
        with: { node-version: ${{ matrix.runtime == 'node-20' && '20.x' || '22.x' }} }
      - name: Setup Bun
        if: startsWith(matrix.runtime, 'bun-')
        uses: oven-sh/setup-bun@v1
        with: { bun-version: "1.1.x" }
      - name: Install via npm
        if: startsWith(matrix.runtime, 'node-')
        shell: bash
        run: |
          mkdir smoke-npm && cd smoke-npm
          npm init -y
          npm install ../tarball/*.tgz
          node -e "const { version } = require('@tinyquant/core'); console.log(version());"
      - name: Install via bun
        if: startsWith(matrix.runtime, 'bun-')
        shell: bash
        run: |
          mkdir smoke-bun && cd smoke-bun
          bun init -y
          bun add ../tarball/*.tgz
          bun -e "import('@tinyquant/core').then(m => console.log(m.version()))"
      - name: Run parity + round-trip tests
        working-directory: javascript/@tinyquant/core
        run: |
          npm ci
          npm install ../../../tarball/*.tgz --no-save
          node --test tests/**/*.test.ts

  pnpm-smoke:
    name: pnpm install smoke
    needs: assemble-tarball
    runs-on: ubuntu-22.04
    steps:
      - uses: actions/checkout@v4
      - uses: actions/download-artifact@v4
        with: { name: npm-tarball, path: ./tarball }
      - uses: actions/setup-node@v4
        with: { node-version: "20.x" }
      - name: Enable pnpm
        run: corepack enable && corepack prepare pnpm@9 --activate
      - run: |
          mkdir smoke-pnpm && cd smoke-pnpm
          pnpm init -y
          pnpm add ../tarball/*.tgz
          node -e "const { version } = require('@tinyquant/core'); console.log(version());"
```

Notes on the workflow:

- `fail-fast: false` on every matrix so a single platform failure
  surfaces all failures instead of masking them.
- The native build step runs napi-rs without triple-name mangling in
  the output — we rename to a stable key so the runtime loader has a
  deterministic path. Doing this in CI (not in `build.rs`) keeps
  `cargo build` uncoupled from the npm packaging convention.
- The install-test job exercises both `npm` and `bun`, on every
  supported runner. That's 15 combinations (5 runners × 3 runtimes).
  `pnpm` is a separate smaller job since it adds package-manager
  coverage without needing to fan out across platforms.
- Parity fixture tests run **inside** the install-test job, so we
  exercise the published tarball and not the source tree.

## Release workflow — `js-release.yml`

```yaml
# .github/workflows/js-release.yml
name: js-release
on:
  push:
    tags: ["v*"]

permissions:
  contents: write      # for GitHub Release upload
  id-token: write      # for npm OIDC provenance

jobs:
  verify-version:
    runs-on: ubuntu-22.04
    steps:
      - uses: actions/checkout@v4
      - name: Check workspace version matches tag and package.json
        shell: bash
        run: |
          TAG="${GITHUB_REF#refs/tags/v}"
          RUST_VER=$(grep -E '^version' rust/Cargo.toml | head -1 | sed -E 's/.*"([0-9.a-z-]+)".*/\1/')
          NPM_VER=$(node -p "require('./javascript/@tinyquant/core/package.json').version")
          echo "tag=$TAG rust=$RUST_VER npm=$NPM_VER"
          test "$TAG" = "$RUST_VER"
          test "$TAG" = "$NPM_VER"

  build-native:
    # same matrix as js-ci.yml
    ...

  publish:
    needs: [verify-version, build-native]
    runs-on: ubuntu-22.04
    steps:
      - uses: actions/checkout@v4
      - uses: actions/download-artifact@v4
        with: { path: javascript/@tinyquant/core/binaries, pattern: native-*, merge-multiple: true }
      - uses: actions/setup-node@v4
        with:
          node-version: "20.x"
          registry-url: "https://registry.npmjs.org"
      - name: TypeScript compile
        working-directory: javascript/@tinyquant/core
        run: |
          npm ci
          npx tsc -p tsconfig.json
          npx tsc -p tsconfig.json --module CommonJS --outDir dist --declaration false
      - name: npm publish with provenance
        working-directory: javascript/@tinyquant/core
        run: npm publish --provenance --access public
      - name: npm pack for GitHub Release
        working-directory: javascript/@tinyquant/core
        run: npm pack --pack-destination ../../../dist-npm
      - name: Upload tarball to GitHub Release
        uses: softprops/action-gh-release@v2
        with:
          files: dist-npm/*.tgz
```

The `verify-version` job is the single choke point for the
[[#Release versioning strategy]] contract. If any of the three
version sources disagree, the publish fails before a single byte
reaches the registry.

## Bun-specific considerations

- `import.meta.dirname` is supported by Bun ≥ 1.1 and Node ≥ 20.11.
  For Node 20.10 and older we fall back via
  `path.dirname(fileURLToPath(import.meta.url))`. The loader code
  handles both.
- Bun's N-API implementation auto-detects `.node` files and loads
  them via its own `bun:ffi` bridge when `require`d. Empirically
  Bun's N-API is one or two patch versions behind the Node.js
  API surface at any given time. The `engines` field pins
  `bun >= 1.1.0`; users on older Bun versions get an
  `EBADENGINE` warning from `bun add`.
- `createRequire(import.meta.url)` works identically on Bun and
  Node — both accept an ESM caller URL and return a CJS
  `require()`. This is the portable entry point for loading
  `.node` files from an ESM module.
- Worker threads: `bun.Worker` is API-compatible with Node's
  `node:worker_threads` for our purposes. Async codec methods
  use napi-rs `AsyncTask`, which Bun schedules on its own thread
  pool, so there's no explicit integration on our side.
- Known Bun gotcha: prior to Bun 1.1, bigint marshalling through
  N-API had a rounding bug for values near `2^53`. Our `u64` seeds
  can exceed this, which is why the minimum Bun version is 1.1.0
  and why `verify-version` in CI pins it explicitly.

## Browser / WASM fallback status

**Explicitly out of scope for Phase 25.** Users on unsupported
platforms (browser, Deno without `--allow-ffi`, musl Linux,
FreeBSD, Windows ARM64) get a clear error at import time:

```
Error: @tinyquant/core: no pre-built binary for <platform>/<arch>.
Supported combinations: linux/x64, linux/arm64, darwin/x64,
darwin/arm64, win32/x64.
```

A future phase can introduce `@tinyquant/core-wasm` as a sibling
package exporting the same TS types (shared via `_types.ts`) backed
by a WebAssembly build of `tinyquant-core`. The distinction is:

- `@tinyquant/core` targets Node and Bun with native `.node`
  binaries and full SIMD.
- `@tinyquant/core-wasm` would target browsers (and fall back for
  unsupported Node/Bun platforms) with reduced SIMD and a portable
  `.wasm` binary.

Phase 25 does not block that future split: the TS API is identical
to what a WASM backend would expose, so adding WASM later is a
matter of a new package with a different loader, not a type surface
change.

## Release versioning strategy

- The npm package version equals the Rust workspace version.
  At first publish that is `0.1.0`, matching `rust/Cargo.toml`
  workspace `version = "0.1.0"` and `tinyquant_cpu` `0.1.1` — the
  Python package is one patch ahead because it predates the Rust
  rewrite; that gap closes at the next minor bump.
- Semver applied consistently: a breaking change in the Rust core
  that surfaces to TS users requires a major bump of every package.
- Pre-release tags use npm's dist-tag convention:
  `0.1.0-rc.1`, `0.1.0-rc.2`, etc. These are published with
  `npm publish --tag next`, so `npm install @tinyquant/core`
  continues to get the last stable release and opt-in users run
  `npm install @tinyquant/core@next`.
- The tag pattern for the release workflow is `v<semver>`, matching
  [[plans/rust/phase-22-pyo3-cabi-release|Phase 22]]'s convention.
- Every publish emits both the npm tarball and a GitHub Release
  asset with an identical tarball, so offline users have a
  deterministic download URL not tied to registry.npmjs.org.

## Rollback plan

npm's rollback story is weaker than PyPI's; a published version
is effectively permanent. The playbook:

1. **Within 72 hours of publish**: `npm unpublish @tinyquant/core@<ver>`
   removes the version entirely. This is allowed only for packages
   published less than 72 hours ago **and** with fewer than 300
   downloads. In practice `--force` is not usable once any consumer
   CI has hit the registry, which happens within minutes.
2. **After 72 hours**: `npm deprecate @tinyquant/core@<ver> "<reason>"`
   is the primary rollback tool. The version stays installable but
   every `npm install` prints the deprecation message. We combine
   this with publishing a new patch version as the fix.
3. **GitHub Release**: mark the tagged release as a draft or
   delete it; delete the attached `.tgz`. Users who downloaded via
   the GitHub Release URL will lose the artifact.
4. **Downstream pin guidance**: the README's install section
   advises pinning an exact version (`@tinyquant/core@0.1.0`)
   rather than a range, so downstreams are not auto-upgraded into
   a broken release. Our own internal consumers use
   package-lock.json / bun.lockb, which we commit.
5. **Provenance revocation**: if a release is compromised, revoke
   the npm OIDC trust configuration for the repo and rotate
   the signing key. Users who verify provenance will see the
   revocation.

## Phase 25.1 declared deviations

The scaffold that landed at commit `a6c80cd` (plus the
workspace-inheritance follow-up) intentionally diverges from the
specification in three places. Each is a forward-compatible
substitution that the remaining sub-slices (25.2–25.4) inherit
transparently.

### 1. napi-rs v2 instead of v3

§`tinyquant-js` crate module map (`Cargo.toml` block at plan
§115–165) pins `napi = "=3.0"` with feature `napi9` and
`napi-derive = "=3.0"`. The scaffold pins `napi = "2"` with
feature `napi8` and `napi-derive = "2"` instead. Rationale: napi-rs
v3 was pre-release at plan-authoring time; v2 is the stable line
that `@napi-rs/cli` 2.x ships against and matches the Node ≥ 20
baseline in §API surface. When napi-rs v3 is GA and passes CI, a
separate maintenance slice can bump the pin — no API change is
required because `#[napi]` attribute macros are stable across v2/v3.

### 2. `moduleResolution: Bundler` instead of `NodeNext`

§`tsconfig.json` (plan §444–475) specifies
`"module": "NodeNext", "moduleResolution": "NodeNext"`. The
scaffold uses `"module": "ESNext", "moduleResolution": "Bundler"`.
Rationale: the Bundler resolution mode was designed precisely for
libraries that ship both ESM and CJS via the `exports` field and is
the setting most downstream TypeScript + bundler toolchains
(`tsup`, `esbuild`, `vite`, `rolldown`) expect to see on a library
published to npm. The `exports` block — which drives runtime
resolution — is unchanged, so Node and Bun see the same entry
points regardless of what this package's own `tsconfig.json` says
for its internal build.

### 3. `./dist/index.cjs` instead of `./dist/index.js` for the CJS entry

§`package.json` (plan §371–443) names the CJS artefact
`./dist/index.js` and the ESM artefact `./dist/index.mjs`. The
scaffold uses `./dist/index.cjs` for CJS and `./dist/index.mjs` for
ESM. Rationale: the explicit `.cjs` extension makes Node's
module-system classification unambiguous under `"type": "module"`
and avoids the class of bug where a consumer-set `"type": "module"`
overrides the per-file default on a file named `index.js`. The
`exports` map still points `require:` at the `.cjs` entry and
`import:` at the `.mjs` entry, so downstream consumers see no
surface change.

## Phase 25.3 declared deviations

### 1. `VectorEntry.metadata` is a no-op in 25.3

§`Corpus.insert(vector_id, vector, metadata=None)` and
§`VectorEntry.metadata` (plan §§916, 925) describe metadata as a
round-trippable `Record<string, unknown>`. In 25.3 the napi-rs
binding accepts the `metadata_json` parameter on the constructor,
`insert`, and `insertBatch` for API-shape parity, but the value is
silently dropped: `EntryMetaValue` (the core's no_std `serde_json`
substitute) is not serde-serialisable from the `tinyquant-js`
crate, and the original `format!("{v:?}")` path in the
`metadata_json` getter produced a data-corrupting rendering
(`{count: 5}` read back as `{"count": "Integer(5)"}`).

Rather than ship that corruption, the getter unconditionally
returns `None` / `null`, matching the PyO3 binding's stance
(`tinyquant_py/src/corpus.rs` raises `NotImplementedError` on
metadata reads/writes, see lines 284–290). The TS wrapper's
`VectorEntry.metadata` getter documents the no-op and returns
`null` regardless of input. Phase 25.4 or beyond wires metadata
through once the core lands a canonical JSON round-trip for
`EntryMetaValue` (either a `serde` impl behind a `alloc-json`
feature flag or a hand-rolled `to_json_string` method on the enum).

No fixture or test depends on metadata round-tripping today, so
the deviation is observable only to callers who pass metadata and
then read it back.

## Steps (TDD order)

- [ ] **Step 1: Scaffold `tinyquant-js` crate.**
  - Create `rust/crates/tinyquant-js/{Cargo.toml,build.rs,package.json,src/lib.rs}`.
  - Add `"crates/tinyquant-js"` to the workspace `members` in
    `rust/Cargo.toml`.
  - Minimal `src/lib.rs`:
    ```rust
    use napi_derive::napi;
    #[napi]
    pub fn version() -> &'static str { env!("CARGO_PKG_VERSION") }
    ```
  - `cargo clippy -p tinyquant-js -- -D warnings` must pass with
    the same deny list as `tinyquant-py`.
  - **Green** when `cargo build -p tinyquant-js --release` produces
    a cdylib and `napi build` in the crate directory produces a
    `.node` file.

- [ ] **Step 2: Scaffold the npm package.**
  - Create `javascript/@tinyquant/core/{package.json,tsconfig.json,src/index.ts,src/_loader.ts,README.md,LICENSE,.npmignore}`.
  - `.npmignore` excludes `tests/`, `node_modules/`, `*.log`.
  - `src/_loader.ts` contains the full loader shown above.
  - `src/index.ts` re-exports `version` from the native binding.
  - **Green** when `npm run build` in the package directory
    produces a `dist/` with `.js`, `.mjs`, and `.d.ts` for each
    source module.

- [ ] **Step 3: Red — first parity test.**
  - Generate fixtures: `python scripts/packaging/generate_js_parity_fixtures.py javascript/@tinyquant/core/tests/fixtures`.
  - Write `tests/parity.test.ts` that loads
    `tests/fixtures/parity/config_hashes.json`, constructs a
    `CodecConfig` for each case in TS, and asserts
    `cfg.configHash === expected.config_hash`.
  - Run `node --test tests/parity.test.ts` — **fails** because
    `CodecConfig` is not yet exposed from the native binding.

- [ ] **Step 4: Implement `src/codec.rs` value objects.**
  - `CodecConfig`, `Codebook`, `CompressedVector`, `RotationMatrix`
    as `#[napi]` classes with `#[napi(constructor)]`, `#[napi(getter)]`,
    `#[napi(factory)]` as appropriate.
  - Delegate every algorithm (SHA-256 hashing, QR decomposition,
    quantile training, etc.) to `tinyquant-core` — no reimplementation.
  - Borrow the `CodecConfig.config_hash` SHA-256 implementation
    exactly: serialize the same canonical string format so the
    hex digests match byte-for-byte across Python and TS.
  - **Green** when `node --test tests/parity.test.ts` passes.

- [ ] **Step 5: Red — round-trip test.**
  - `tests/round-trip.test.ts` generates 10 000 f32 vectors from
    a seeded PRNG in TS, compresses + decompresses each, and
    asserts `MSE < 1e-2` against the originals. Fails until
    `compress`/`decompress` are exposed.

- [ ] **Step 6: Implement `Codec.compress` / `Codec.decompress`.**
  - Buffer conversion via `src/buffers.rs` — `Float32Array::as_ref()`
    for zero-copy input, `Float32Array::new(vec)` for output.
  - Dimension and bit-width validation at the boundary, raising
    `TinyQuantError` via `src/errors.rs`.
  - Module-level convenience `compress` / `decompress` functions
    that construct a default `Codec` (mirroring Python module fns).
  - **Green** when round-trip test passes on the dev machine.

- [ ] **Step 7: Implement corpus + backend modules.**
  - `src/corpus.rs` exposes `CompressionPolicy` (as a class with
    static factory methods since napi-rs doesn't map Rust enums
    directly to JS consts the way PyO3 does), `VectorEntry`, and
    `Corpus`.
  - `src/backend.rs` exposes `SearchResult` and
    `BruteForceBackend`.
  - `pendingEvents()` serializes corpus events into plain JS
    objects (discriminated by a `type` string).
  - **Green** when `tests/corpus.test.ts` and
    `tests/backend.test.ts` pass, each backed by the
    corresponding fixture file.

- [ ] **Step 8: TypeScript type surface.**
  - `napi build --dts` emits `dist/index.d.ts` from the annotations.
  - Add hand-written JSDoc in the wrapper files
    (`src/codec.ts`, `src/corpus.ts`, `src/backend.ts`).
  - Ship both `.d.ts` (ESM-style) and `.d.cts` (CJS-style)
    under `exports` so TypeScript resolves types correctly for
    both module systems.

- [ ] **Step 9: `js-ci.yml` workflow.**
  - Implement the matrix exactly as specified in
    [[#CI workflow — js-ci.yml]].
  - Gate merges to `main` on the workflow passing.

- [ ] **Step 10: Documentation.**
  - `README.md` at `javascript/@tinyquant/core/README.md` — install
    instructions, quickstart, API reference link to the Obsidian
    wiki.
  - Top-level `.github/README.md` gains a "Language bindings"
    section cross-linking Python, Rust, and TypeScript packages.

- [ ] **Step 11: `js-release.yml` workflow.**
  - Tag-triggered publish with provenance, GitHub Release asset
    upload, and the version-sync pre-check.

- [ ] **Step 12: First publish.**
  - Tag `v0.1.0-rc.1` and publish to the `next` dist-tag.
  - Install into a clean workspace on each platform (manual
    dogfooding beyond CI) and verify the runtime loader path.
  - When satisfied, tag `v0.1.0` for the `latest` dist-tag.

## Acceptance criteria

- [ ] `npm install @tinyquant/core` succeeds on `linux/x64`,
      `linux/arm64`, `darwin/x64`, `darwin/arm64`, `win32/x64`.
- [ ] `bun add @tinyquant/core` succeeds on the same 5 platforms.
- [ ] `pnpm add @tinyquant/core` succeeds (smoke test only).
- [ ] `tests/parity.test.ts` passes on every CI matrix cell:
      every `config_hash` fixture is byte-identical between
      Python and TS.
- [ ] Round-trip on 10 000 random f32 vectors passes with
      `MSE < 1e-2` on every `(platform, runtime)` pair.
- [ ] `npm publish --provenance` emits a valid Sigstore
      attestation; the published version shows the
      "Verified" badge on npmjs.com.
- [ ] Package size ≤ 10 MB gzipped, ≤ 50 MB uncompressed.
- [ ] Zero runtime dependencies that require compilation on the
      user's machine (`node-gyp` never invoked).
- [ ] TypeScript `strict: true` compiles cleanly against the
      published `.d.ts`; an example consumer at
      `examples/typescript-consumer/` builds and runs.
- [ ] `verify-version` pre-check rejects a tag whose semver does
      not equal both `rust/Cargo.toml` and `package.json`.
- [ ] Runtime loader returns a descriptive error for unsupported
      platforms (verified by unit test that monkey-patches
      `process.platform` / `process.arch`).
- [ ] The runtime loader does **not** call `child_process.exec` or
      any similar subprocess API (enforced by a repo-root
      `scripts/check_no_exec.sh` grep step in CI).

## Risks

- **napi-rs v2 → v3 churn.** Breaking changes between napi-rs major
  versions have historically bitten downstream packages. *Mitigation*:
  pin `napi` and `napi-derive` to exact versions (`=3.0`), audit
  `CHANGELOG.md` before every bump, and run a full CI matrix on a
  dedicated branch before merging the bump.
- **Bun N-API lag.** Bun's N-API implementation trails Node's by
  1–2 patch versions. *Mitigation*: pin `bun >= 1.1.0` in
  `engines`, run every test against the current stable Bun in CI,
  and keep a fallback path that emits a clear error if a specific
  N-API feature is unavailable under Bun.
- **glibc vs musl.** Our prebuilt Linux binaries target glibc; Alpine
  users will see the "no pre-built binary" error. *Mitigation*: clear
  error message with a link to the tracking issue; consider adding an
  Alpine build in a follow-up phase. The loader's `binaryKey()` already
  distinguishes `linux-x64-gnu` vs `linux-x64-musl` so adding the
  binary is purely additive.
- **Bundled binaries vs optional-deps convention.** The bundled
  strategy diverges from napi-rs's `@scope/pkg-<triple>` default,
  which means some ecosystem tooling (e.g. `pnpm`'s optional-deps
  skipping) doesn't apply. *Mitigation*: CI exercises `pnpm`
  explicitly, and we document the trade-off in the package README
  for downstream debugging. If the pattern proves problematic we
  fall back to the optional-deps variant **without** changing the
  public API — the consumer surface is identical either way.
- **Typed-array lifetime bugs.** Holding a `Float32Array::as_ref()`
  borrow past the end of the sync call is undefined behavior. *Mitigation*:
  every async method copies into a `Vec<f32>` before spawning; unit
  tests exercise rapid interleaved sync/async calls with large
  arrays to surface any regression; `cargo miri` is part of
  `tinyquant-js` CI.
- **Version drift between package.json, Cargo.toml, and git tag.**
  *Mitigation*: the `verify-version` release-workflow job blocks
  publish on any mismatch. Developers run `scripts/bump_versions.sh`
  (Phase 22) which updates all three sources atomically.
- **npm registry outage at release time.** Transient publish
  failures leave the repo tagged but not published. *Mitigation*:
  the release workflow is idempotent on re-run (`npm publish`
  skips if the version already exists and will `exit 0`); the
  GitHub Release asset is the authoritative artifact if npm is
  unreachable, so enterprise users can always fall back to it.

## Open questions

- **Package name scope.** `@tinyquant/core` (scoped, leaves room
  for future `@tinyquant/cli`, `@tinyquant/pgvector`) vs
  `tinyquant` (unscoped). The plan goes with `@tinyquant/core`;
  we still need to **register the scope on npm** before the first
  publish.
- **Bun vs Node as first-class runtime.** Currently both are
  equal. If Bun's N-API divergence forces per-runtime patches
  more than twice, we drop Bun to "best effort" status and
  document Node as the supported runtime.
- **Streaming corpus events.** Python emits domain events via
  `pending_events()`. TS could expose an `AsyncIterable<CorpusEvent>`
  or a Node `EventEmitter`. Deferred to Step 7 detail design;
  the initial surface uses the `pendingEvents()` drain pattern
  identical to Python so there's no semantic drift at launch.
- **SharedArrayBuffer support.** Batch methods could accept
  SABs for cross-Worker zero-copy. Deferred until a concrete
  consumer requests it.

## See also

- [[research/python-rust-wrapper-distribution|Distribution Research]]
- [[plans/rust/phase-22-pyo3-cabi-release|Phase 22: PyO3, C ABI, Release]]
- [[plans/rust/phase-24-python-fat-wheel-official|Phase 24: Python Fat Wheel Official]]
- napi-rs: https://napi.rs/
- Bun N-API docs: https://bun.sh/docs/runtime/nodejs-apis
- npm provenance: https://docs.npmjs.com/generating-provenance-statements
- N-API stability: https://nodejs.org/api/n-api.html#node-api-version-matrix
