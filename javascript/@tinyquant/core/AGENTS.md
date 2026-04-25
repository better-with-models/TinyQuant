# AGENTS.md — Guide for AI Agents Working in `@tinyquant/core`

> *CPU-only vector quantization codec for embedding storage compression.*

This directory is the shipping npm package. It wraps the Rust
`tinyquant-js` crate (napi-rs bindings over `tinyquant-core`) into
an ESM + CJS TypeScript package with pre-built native binaries for
five platforms.

## What this area contains

- **Primary responsibility:** TypeScript wrappers over the native
  binding, plus the build pipeline that produces `dist/` and bundles
  one `.node` per supported triple into `binaries/`.
- **Main entrypoints:**
  - `src/index.ts` — public barrel; re-exports codec / corpus /
    backend surfaces.
  - `src/_loader.ts` — platform-triple selector that loads the
    correct `binaries/<key>.node`.
  - `scripts/build.mjs` — orchestrates napi build → rename → tsc
    (ESM) → emit-cjs-types → emit-cjs-bundle.
  - `package.json` — the single source of truth for `engines`,
    `exports`, `files`, and the `build` / `test` scripts.
- **Common changes:** adding wrapper code for new Rust exports,
  adjusting `exports` / `files`, extending the test corpus, and
  maintaining parity fixtures under `tests/fixtures/`.

## Layout

```text
@tinyquant/core/
├── src/              TypeScript wrappers + loader
│   ├── index.ts      public barrel
│   ├── _loader.ts    platform-triple selector
│   ├── _errors.ts    TinyQuantError + native-error parsing
│   ├── codec.ts      Codec / CodecConfig / Codebook / ...
│   ├── corpus.ts     Corpus / CompressionPolicy / VectorEntry
│   └── backend.ts    BruteForceBackend / SearchResult
├── binaries/         per-triple .node files (built or downloaded)
├── dist/             tsc output: *.js, *.cjs, *.d.ts, *.d.cts, *.map
├── scripts/          build-pipeline .mjs drivers
│   ├── build.mjs
│   ├── rename-binaries.mjs
│   ├── emit-cjs-types.mjs
│   └── emit-cjs-bundle.mjs
├── tests/            node:test suites (.ts + .cjs)
│   └── fixtures/     Python-generated parity fixtures
├── tsconfig.json     primary ESM tsconfig
├── tsconfig.cjs.json CJS-typings tsconfig variant
├── tsconfig.test.json tsc → dist-tests/ for node:test runs
└── package.json      public npm manifest
```

## Test / build commands

```bash
# Full build: napi build → rename → tsc ESM → emit CJS types → emit CJS bundle.
npm run build

# Full test: build + compile tests + node --test (parity + round-trip
# + corpus + backend + types + cjs-smoke).
npm test

# Regenerate parity fixtures (requires a working Python reference env).
npm run fixtures:generate
```

## Invariants — Do Not Violate

- **Binary layout.** `binaries/<triple>.node` is the authoritative
  layout (NOT the Python fat wheel's `_lib/<key>/` tree). The loader
  is hard-coded to this shape.
- **Math delegation.** TypeScript wrappers NEVER reimplement codec
  math. Every algorithm (config hashing, QR rotation, quantile
  training, bit packing) delegates to the native binding. The
  wrappers exist only for ergonomics (camelCase, JSDoc, and
  branded error classes).
- **CJS bundle integrity.** `scripts/emit-cjs-bundle.mjs` produces
  a real runtime `.cjs` from the ESM output, not a re-export
  shim. Any edit to it MUST be followed by `npm test` to re-run the
  CJS smoke test (`tests/cjs-smoke.test.cjs`).
- **napi-rs version pin.** `@napi-rs/cli` and the napi-derive crate
  are declared-deviations: plan specifies v3, we ship v2. Do not
  "fix" this without raising a follow-up slice — see plan §Phase 25.1
  declared deviations.
- **Version lockstep.** `package.json` `version` MUST equal
  `rust/Cargo.toml` `workspace.package.version`. The
  `js-release.yml::verify-version` job gates publish on this.
- **No runtime compilation.** `node-gyp` / `cargo` must never run
  on the user's machine. All five `.node` binaries are pre-built
  in CI and bundled into the tarball.
- **VectorEntry.metadata is a no-op.** The getter returns `null`
  regardless of input. This is a declared Phase 25.3 deviation.

## Common workflows

### Update existing behavior

1. Read the file you intend to touch + `src/index.ts` (to understand
   what the export surface commits to).
2. Follow the invariants above before introducing new files or
   algorithms.
3. Run `npm test` — never skip the CJS smoke step.
4. Update the Obsidian wiki entry under `docs/` if the public API
   surface changes.

### Add a new wrapper

1. Confirm the Rust `#[napi]` binding already exists in
   `rust/crates/tinyquant-js/src/`. If not, land that slice first.
2. Add the TS wrapper in the matching `src/<module>.ts`, re-export
   via `src/index.ts`.
3. Add a fixture-backed test under `tests/`.
4. Run `npm test` to compile + execute the new test.

## See also

- [Parent AGENTS.md](../../../AGENTS.md) — repo-level operating
  contract
- [`rust/crates/tinyquant-js/AGENTS.md`](../../../rust/crates/tinyquant-js/AGENTS.md)
  — the sibling Rust crate that produces the native binding
- [`docs/plans/rust/phase-25-typescript-npm-package.md`](../../../docs/plans/rust/phase-25-typescript-npm-package.md)
  — authoritative plan
- [`docs/design/rust/phase-25-implementation-notes.md`](../../../docs/design/rust/phase-25-implementation-notes.md)
  — implementation notes + AC trace
