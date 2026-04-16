---
title: "Phase 25.5: JS/TS Test Gap Remediation"
tags:
  - plans
  - javascript
  - typescript
  - phase-25.5
  - testing
  - gaps
date-created: 2026-04-15
status: draft
category: planning
---

# Phase 25.5: JS/TS Test Gap Remediation

> [!info] Goal
> Close every P1 and P2 JS/TS testing gap identified in
> [[requirements/testing-gaps|testing-gaps.md]] §GAP-JS-*.
> No new features ship. Deliverables are tests, CI steps, and
> small shell scripts — no changes to `tinyquant-js` production source
> or the public TypeScript API surface.
>
> GAP-JS-006 (musl binaries) is the one exception: adding musl
> cross-compilation requires a new CI matrix cell and a corresponding
> binary in the npm tarball, which is a production artifact change.
> It is included here because the loader already handles the musl
> key — the binary is the only missing piece.
>
> This phase runs in parallel with Phase 26 (Rust calibration gates).
> Neither depends on the other.

> [!note] Reference docs
> - [[requirements/testing-gaps|Testing Gaps]] — §GAP-JS-004, GAP-JS-002, GAP-JS-006–010
> - [[requirements/js|JS/TS Requirements]] — FR-JS-002, FR-JS-004, FR-JS-006–010
> - [[plans/rust/phase-25-typescript-npm-package|Phase 25]] — existing JS package structure
> - [[requirements/corpus|Corpus Requirements]] — FR-CORP-003, FR-CORP-006, FR-CORP-007 (re-tested via N-API)

## Prerequisites

- Phase 25 complete: `@tinyquant/core` published to npm, N-API binding
  functional, all five glibc platform binaries built and bundled, `js-ci.yml`
  passing.
- `javascript/@tinyquant/core/tests/` test suite green locally:
  `parity.test.ts`, `round-trip.test.ts`, `backend.test.ts`,
  `types.test.ts`, `corpus.test.ts`, `loader.test.ts`.
- `napi` CLI ≥ 3.x installed in the CI environment; Rust musl toolchain
  (`x86_64-unknown-linux-musl`, `aarch64-unknown-linux-musl`) available
  via `rustup target add`.

## Scope

Parts A–D add or extend TypeScript tests. Parts E–F add or confirm shell
script CI wiring. Part G adds musl cross-compilation to the CI matrix.

No changes to `rust/crates/tinyquant-js/src/` unless a newly failing test
reveals a genuine bug; any such fix is committed separately with a `fix:`
prefix.

## Deliverables

### Files to modify or create

| File | Gaps closed | Notes |
|---|---|---|
| `javascript/@tinyquant/core/tests/round-trip.test.ts` | GAP-JS-002 | Add dim=768 round-trip `it()` block |
| `javascript/@tinyquant/core/tests/corpus.test.ts` | GAP-JS-004 | Add three N-API corpus policy invariant test cases |
| `javascript/@tinyquant/core/tests/esm-subpath-smoke.test.ts` (new) | GAP-JS-007 | Sub-path exports smoke test |
| `.github/workflows/js-ci.yml` | GAP-JS-006, GAP-JS-008, GAP-JS-009, GAP-JS-010 | musl matrix cell + size gate + version check + no-exec check |
| `scripts/check_version_consistency.sh` (new or extend) | GAP-JS-009 | Extract version comparison logic from js-release.yml |
| `scripts/check_no_exec.sh` (verify/add) | GAP-JS-010 | Scan compiled output for subprocess-spawning API patterns |

---

## Part A — dim=768 round-trip coverage (GAP-JS-002)

The existing `round-trip.test.ts` tests N=10 000, dim=128. A dim=768
regression at the N-API boundary would be invisible today.

### Step 1 — Failing test (red)

Add to `javascript/@tinyquant/core/tests/round-trip.test.ts`:

```typescript
// GAP-JS-002: dim=768 round-trip must achieve MSE < 1e-2
// Uses N=1000 to keep the test under 5 seconds. Codebook is trained on
// the first 256 vectors; the remaining 744 are test vectors.
it("round-trips N=1000, dim=768 vectors with MSE < 1e-2", () => {
  const dim = 768;
  const N = 1000;
  const seed = 0xdeadbeef;

  const config = new CodecConfig({ bitWidth: 4, seed, dim, residual: false });

  // Deterministic training corpus: 256 vectors using the seeded generator.
  const trainingVectors: Float32Array[] = Array.from({ length: 256 }, (_, i) =>
    seededVector(dim, seed ^ i)
  );
  const codebook = Codebook.train(config, trainingVectors);

  let totalMse = 0;
  for (let i = 0; i < N; i++) {
    const v = seededVector(dim, seed ^ (i + 256));
    const cv = compress(v, config, codebook);
    const decompressed = decompress(cv, config, codebook);
    totalMse += mse(v, decompressed);
  }
  const meanMse = totalMse / N;

  expect(meanMse).toBeLessThan(
    1e-2,
    `mean MSE for dim=768 round-trip was ${meanMse.toFixed(6)}, expected < 1e-2`
  );
});
```

> [!note] `seededVector` helper
> `seededVector(dim, seed)` generates a deterministic `Float32Array` via
> a simple xorshift32 from `seed`. If this helper does not exist in the
> test file, add it at the top of the file:
>
> ```typescript
> function seededVector(dim: number, seed: number): Float32Array {
>   let s = seed >>> 0;
>   const out = new Float32Array(dim);
>   for (let i = 0; i < dim; i++) {
>     s ^= s << 13; s ^= s >> 17; s ^= s << 5;
>     out[i] = ((s >>> 0) / 0xffffffff) * 2 - 1; // [-1, 1]
>   }
>   return out;
> }
> ```
>
> Similarly, `mse(a, b)` computes mean squared error:
>
> ```typescript
> function mse(a: Float32Array, b: Float32Array): number {
>   let sum = 0;
>   for (let i = 0; i < a.length; i++) sum += (a[i] - b[i]) ** 2;
>   return sum / a.length;
> }
> ```

### Step 2 — Make test pass

No production code changes expected. If the test fails with MSE ≥ 1e-2,
diagnose whether the codebook training path is truncating dim or whether
the N-API `Float32Array` bridge is copying incorrectly. The Rust bench
already passes dim=1536 at the same `Must` threshold, so a failure here
is a JS-layer bug, not a core codec regression.

---

## Part B — Corpus policy invariants through the N-API boundary (GAP-JS-004)

The Rust and Python layers both test cross-config rejection, FP16 precision,
and policy immutability. None of these is exercised through the N-API binding.

### Step 3 — Failing tests (red)

Add to `javascript/@tinyquant/core/tests/corpus.test.ts`:

```typescript
// GAP-JS-004 (1/3): cross-config insertion rejection through N-API
it("rejects insertion with a mismatched config hash", () => {
  const configA = new CodecConfig({ bitWidth: 4, seed: 1, dim: 64 });
  const configB = new CodecConfig({ bitWidth: 4, seed: 2, dim: 64 }); // different seed → different hash
  const codebookA = Codebook.train(configA, trainingVectors(64, 256, 1));
  const codebookB = Codebook.train(configB, trainingVectors(64, 256, 2));

  const corpus = new Corpus(configA, codebookA, "Compress");
  corpus.insert("v0", seededVector(64, 10));

  // Inserting a vector compressed under configB should throw.
  expect(() => {
    const cvB = compress(seededVector(64, 11), configB, codebookB);
    corpus.insertCompressed("v1", cvB);
  }).toThrow(/ConfigMismatchError/);
});

// GAP-JS-004 (2/3): policy immutability after first insertion through N-API
it("rejects policy change on a non-empty corpus", () => {
  const config = new CodecConfig({ bitWidth: 4, seed: 42, dim: 64 });
  const codebook = Codebook.train(config, trainingVectors(64, 256, 42));
  const corpus = new Corpus(config, codebook, "Compress");
  corpus.insert("v0", seededVector(64, 0));

  expect(() => {
    corpus.setCompressionPolicy("Passthrough");
  }).toThrow(/PolicyImmutableError/);
});

// GAP-JS-004 (3/3): FP16 policy precision bound through N-API
it("FP16 policy preserves each element within 2^-13 × |original|", () => {
  const dim = 64;
  const config = new CodecConfig({ bitWidth: 4, seed: 42, dim });
  const codebook = Codebook.train(config, trainingVectors(dim, 256, 42));
  const corpus = new Corpus(config, codebook, "Fp16");

  const original = seededVector(dim, 7);
  corpus.insert("v0", original);
  const decompressed = corpus.decompressAll().get("v0")!;

  for (let i = 0; i < dim; i++) {
    const o = original[i];
    const r = decompressed[i];
    if (o === 0) {
      expect(r).toBe(0);
      continue;
    }
    const bound = Math.pow(2, -13) * Math.abs(o);
    expect(Math.abs(o - r)).toBeLessThanOrEqual(
      bound,
      `element ${i}: |${o} - ${r}| exceeds FP16 precision bound ${bound}`
    );
  }
});
```

> [!note] `trainingVectors` helper
> If not already present in `corpus.test.ts`, add:
>
> ```typescript
> function trainingVectors(
>   dim: number,
>   count: number,
>   seedBase: number
> ): Float32Array[] {
>   return Array.from({ length: count }, (_, i) =>
>     seededVector(dim, seedBase ^ i)
>   );
> }
> ```

### Step 4 — Make tests pass

These tests exercise existing N-API surface (`corpus.insert`,
`corpus.setCompressionPolicy`, `corpus.decompressAll`). If any test fails
with an unexpected error type or missing method, check whether the N-API
binding exports the method and whether the JS error type string matches
the pattern in the `toThrow` matcher.

If `corpus.insertCompressed` does not exist (because the Phase 25
binding does not expose raw `CompressedVector` insertion), replace
the cross-config test with a scenario that drives the codec through the
`corpus.insert` path with a `Corpus` configured with configA, and
attempts to call `corpus.insert` with a vector produced by `Corpus`
configured with configB — the binding should detect the config hash
mismatch on the Rust side and throw.

---

## Part C — Sub-path ESM exports smoke test (GAP-JS-007)

The test suite imports only from the package root (`@tinyquant/core`).
A broken sub-path entry in `package.json` `"exports"` field would be
invisible.

### Step 5 — Create `esm-subpath-smoke.test.ts`

```typescript
// javascript/@tinyquant/core/tests/esm-subpath-smoke.test.ts
//
// GAP-JS-007: verify that each documented sub-path export resolves
// and exports a constructor, not undefined or a plain object.
//
// This test is deliberately shallow: it only checks that the import
// resolves and that the named export is a function (JS class). It
// does not instantiate the classes (that is covered by the other test
// files). A broken "exports" entry throws at import time with
// ERR_PACKAGE_PATH_NOT_EXPORTED, which fails the test immediately.

import { CodecConfig }        from "@tinyquant/core/codec";
import { Corpus }             from "@tinyquant/core/corpus";
import { BruteForceBackend }  from "@tinyquant/core/backend";

describe("sub-path exports", () => {
  it("@tinyquant/core/codec exports CodecConfig as a constructor", () => {
    expect(typeof CodecConfig).toBe("function");
  });

  it("@tinyquant/core/corpus exports Corpus as a constructor", () => {
    expect(typeof Corpus).toBe("function");
  });

  it("@tinyquant/core/backend exports BruteForceBackend as a constructor", () => {
    expect(typeof BruteForceBackend).toBe("function");
  });
});
```

### Step 6 — Make tests pass

If any import fails with `ERR_PACKAGE_PATH_NOT_EXPORTED`, add the
missing entry to the `"exports"` field in
`javascript/@tinyquant/core/package.json`:

```json
"exports": {
  ".":        { "import": "./dist/index.mjs", "require": "./dist/index.cjs" },
  "./codec":   { "import": "./dist/codec.mjs",  "require": "./dist/codec.cjs" },
  "./corpus":  { "import": "./dist/corpus.mjs", "require": "./dist/corpus.cjs" },
  "./backend": { "import": "./dist/backend.mjs","require": "./dist/backend.cjs" }
}
```

Adjust the `dist/` paths to match whatever the Phase 25 TypeScript build
actually emits. If the build does not produce per-module output files,
consider adding a `tsup` or `rollup` split-entry build step rather than
re-exporting everything from index.

---

## Part D — Package size gate (GAP-JS-008)

### Step 7 — Add size-gate step in `js-ci.yml` (red: missing)

The absence of the step is the failing state. Adding the step closes the gap.

Add to `.github/workflows/js-ci.yml` after the `npm run build` step:

```yaml
- name: Check package size (gzip ≤ 10 MB, unpacked ≤ 50 MB)
  working-directory: javascript/@tinyquant/core
  run: |
    PACK_OUTPUT=$(npm pack --dry-run 2>&1)
    echo "$PACK_OUTPUT"

    # Extract sizes — npm prints lines like:
    #   package size:  4.1 MB
    #   unpacked size: 8.5 MB
    GZIP_MB=$(echo "$PACK_OUTPUT" \
      | grep -i "package size" \
      | grep -oE '[0-9]+\.[0-9]+' | head -1)
    UNPACK_MB=$(echo "$PACK_OUTPUT" \
      | grep -i "unpacked size" \
      | grep -oE '[0-9]+\.[0-9]+' | head -1)

    echo "Gzip: ${GZIP_MB} MB  Unpacked: ${UNPACK_MB} MB"

    python3 - <<'PYEOF'
    import sys, os
    gzip   = float(os.environ.get("GZIP_MB",   "0") or sys.argv[1] if len(sys.argv)>1 else "0")
    unpack = float(os.environ.get("UNPACK_MB", "0") or sys.argv[2] if len(sys.argv)>2 else "0")
    PYEOF

    if (( $(echo "$GZIP_MB > 10" | bc -l) )); then
      echo "FAIL: gzip size ${GZIP_MB} MB exceeds 10 MB limit"
      exit 1
    fi
    if (( $(echo "$UNPACK_MB > 50" | bc -l) )); then
      echo "FAIL: unpacked size ${UNPACK_MB} MB exceeds 50 MB limit"
      exit 1
    fi
    echo "PASS: package size within budget"
```

> [!note] Threshold rationale
> The Phase 25 plan documents the expected size as ~4 MB gzip / ~8.5 MB
> unpacked for five glibc binaries. Adding two musl binaries (Part G of
> this phase) adds approximately 3 MB gzip, keeping the total around 7 MB.
> The 10 MB gate gives headroom before the next architecture is added
> and ensures a deliberate decision if the threshold is approached.

### Step 8 — Verify CI job passes

Run the workflow on a branch with no binary changes. The step should
report the current size and pass. If it fails due to an existing
over-budget size, do not relax the threshold — investigate what is
contributing to the tarball and trim before this phase is declared complete.

---

## Part E — Version consistency check on PRs (GAP-JS-009)

The `verify-version` step in `js-release.yml` is released-tag-only.
A PR can merge with a bumped `package.json` but an un-bumped `Cargo.toml`
(or vice versa) and only fail at release time.

### Step 9 — Create `scripts/check_version_consistency.sh`

```bash
#!/usr/bin/env bash
# scripts/check_version_consistency.sh
#
# GAP-JS-009: verify that package.json, Cargo.toml, and (if present)
# js-release.yml expected-version all agree.
#
# Usage: bash scripts/check_version_consistency.sh
# Exit 0 if all versions agree; exit 1 with a descriptive message if not.
set -euo pipefail

JS_PKG="javascript/@tinyquant/core/package.json"
CARGO_TOML="rust/crates/tinyquant-js/Cargo.toml"

if [[ ! -f "$JS_PKG" ]]; then
  echo "ERROR: $JS_PKG not found" >&2
  exit 1
fi
if [[ ! -f "$CARGO_TOML" ]]; then
  echo "ERROR: $CARGO_TOML not found" >&2
  exit 1
fi

PKG_VERSION=$(python3 -c "import json,sys; d=json.load(open(sys.argv[1])); print(d['version'])" "$JS_PKG")
CARGO_VERSION=$(grep -m1 '^version' "$CARGO_TOML" | sed 's/.*"\(.*\)".*/\1/')

echo "package.json version : $PKG_VERSION"
echo "Cargo.toml version   : $CARGO_VERSION"

if [[ "$PKG_VERSION" != "$CARGO_VERSION" ]]; then
  echo ""
  echo "FAIL: version mismatch — package.json=$PKG_VERSION Cargo.toml=$CARGO_VERSION"
  echo "Both files must carry the same version before merging."
  exit 1
fi

echo "PASS: versions match ($PKG_VERSION)"
```

Make the script executable in the repo:

```bash
chmod +x scripts/check_version_consistency.sh
```

### Step 10 — Wire into `js-ci.yml`

Add to `.github/workflows/js-ci.yml`, in a job that runs on PRs touching
`javascript/@tinyquant/core/**` or `rust/crates/tinyquant-js/**`:

```yaml
- name: Check version consistency (package.json ↔ Cargo.toml)
  run: bash scripts/check_version_consistency.sh
```

Place this step before the build step so the job fails fast on a version
mismatch without waiting for compilation.

---

## Part F — No-subprocess loader check (GAP-JS-010)

### Step 11 — Verify or add `scripts/check_no_exec.sh`

The Phase 25 plan documents `scripts/check_no_exec.sh` as a CI step, but
its presence in `js-ci.yml` is unconfirmed. This step closes the gap with
either a confirmation or an addition.

**Check first:** inspect `.github/workflows/js-ci.yml` for a step calling
`check_no_exec.sh`. If the step is present and the script exists, GAP-JS-010
is already closed — document this in the gap's `Gap:` field in
`docs/requirements/js.md` and mark it `None.` without making further changes.

**If the step is absent**, create `scripts/check_no_exec.sh`:

```bash
#!/usr/bin/env bash
# scripts/check_no_exec.sh
#
# GAP-JS-010: scan a directory for imports/requires of subprocess-spawning
# Node.js APIs. The TinyQuant loader must not invoke child processes.
#
# Usage: bash scripts/check_no_exec.sh <directory>
# Exit 0 if no forbidden patterns found; exit 1 with matches listed.
set -euo pipefail

TARGET="${1:?Usage: $0 <directory>}"

# Patterns that indicate subprocess invocation in compiled JS/TS output.
PATTERNS=(
  'require("child_process")'
  "require('child_process')"
  'from "child_process"'
  "from 'child_process'"
  'spawnSync'
  'execSync'
  'execFileSync'
)

FOUND=0
for PATTERN in "${PATTERNS[@]}"; do
  MATCHES=$(grep -r --include="*.js" --include="*.mjs" --include="*.cjs" \
    -l "$PATTERN" "$TARGET" 2>/dev/null || true)
  if [[ -n "$MATCHES" ]]; then
    echo "FAIL: found '$PATTERN' in:"
    echo "$MATCHES" | sed 's/^/  /'
    FOUND=1
  fi
done

if [[ "$FOUND" -eq 1 ]]; then
  echo ""
  echo "Loader must not use child_process or synchronous spawn/exec APIs."
  exit 1
fi

echo "PASS: no subprocess-spawning patterns found in $TARGET"
```

Then add to `.github/workflows/js-ci.yml` after the build step:

```yaml
- name: Verify loader has no subprocess invocations
  run: bash scripts/check_no_exec.sh javascript/@tinyquant/core/dist
```

---

## Part G — musl Linux binary cross-compilation (GAP-JS-006)

This is the only deliverable that changes the npm artifact: two new
`.node` binaries are added to `javascript/@tinyquant/core/binaries/`.

### Step 12 — Add musl targets to the CI build matrix

In `.github/workflows/js-ci.yml`, the matrix that builds platform binaries
currently has five entries (per Phase 25). Add two musl entries:

```yaml
matrix:
  include:
    # Existing glibc targets (Phase 25)
    - target: x86_64-unknown-linux-gnu
      os: ubuntu-22.04
    - target: aarch64-unknown-linux-gnu
      os: ubuntu-22.04
    - target: x86_64-apple-darwin
      os: macos-13
    - target: aarch64-apple-darwin
      os: macos-14
    - target: x86_64-pc-windows-msvc
      os: windows-2022

    # Phase 25.5: musl Linux targets
    - target: x86_64-unknown-linux-musl
      os: ubuntu-22.04
      musl: true
    - target: aarch64-unknown-linux-musl
      os: ubuntu-22.04
      musl: true
```

In the build step, add a conditional `rustup target add` for musl:

```yaml
- name: Add musl cross-compilation toolchain
  if: matrix.musl == true
  run: |
    rustup target add ${{ matrix.target }}
    # musl-cross provides the C linker for musl targets on glibc hosts.
    sudo apt-get install -y musl-tools
    # aarch64 musl additionally needs a cross linker.
    if [[ "${{ matrix.target }}" == "aarch64-unknown-linux-musl" ]]; then
      sudo apt-get install -y gcc-aarch64-linux-gnu
    fi
```

The `napi build` invocation already uses `--target ${{ matrix.target }}`;
no other change is needed in the build command.

### Step 13 — Update `loader.ts` binary key comment

The loader already maps `linux`+`musl` → `linux-x64-musl` /
`linux-arm64-musl`. After the binaries exist, remove the `// NOT YET
BUNDLED` comment (if present) from `src/_loader.ts` and update the
table of supported platform keys in the JSDoc block.

### Step 14 — Verify loader behavior on musl

Add to `javascript/@tinyquant/core/tests/loader.test.ts` (or confirm it
already exists — check before adding):

```typescript
// GAP-JS-006: musl platform keys must resolve to a binary key
// (actual file load is tested in the CI matrix job; this unit test
// verifies the key computation logic only).
it("computes 'linux-x64-musl' key for linux/x64 on musl", () => {
  expect(binaryKey("linux", "x64", /* isMusl */ true)).toBe("linux-x64-musl");
});

it("computes 'linux-arm64-musl' key for linux/arm64 on musl", () => {
  expect(binaryKey("linux", "arm64", /* isMusl */ true)).toBe(
    "linux-arm64-musl"
  );
});
```

These tests verify the key logic in isolation; the actual `.node` file
load is verified by the matrix build job when it runs the full test suite
on a musl runner (Alpine-based container).

> [!note] musl detection at runtime
> The loader determines `isMusl` by reading `/proc/self/maps` or by
> checking whether `process.report.getReport().header.glibcVersionRuntime`
> is `undefined`. The exact detection heuristic is documented in
> `src/_loader.ts`. The unit test above mocks `isMusl` directly via the
> `binaryKey` function's signature — the detection logic itself is tested
> by the integration run on the Alpine matrix job.

---

## Steps summary (ordered)

| Step | Gap(s) closed | File | Est. effort |
|---|---|---|---|
| 1–2 | GAP-JS-002 | `round-trip.test.ts` | 1 h |
| 3–4 | GAP-JS-004 | `corpus.test.ts` | 1.5 h |
| 5–6 | GAP-JS-007 | `esm-subpath-smoke.test.ts` (new) | 0.5 h |
| 7–8 | GAP-JS-008 | `js-ci.yml` (size gate) | 0.5 h |
| 9–10 | GAP-JS-009 | `scripts/check_version_consistency.sh`, `js-ci.yml` | 0.5 h |
| 11 | GAP-JS-010 | `scripts/check_no_exec.sh` (verify/add), `js-ci.yml` | 0.5 h |
| 12–14 | GAP-JS-006 | `js-ci.yml` (musl matrix), `_loader.ts` comment, `loader.test.ts` | 2 h |

---

## Acceptance criteria

- [ ] `npm test` green in `javascript/@tinyquant/core/` with the three
      new test cases in `corpus.test.ts` passing.
- [ ] `round-trip.test.ts` dim=768 `it()` block passes (MSE < 1e-2).
- [ ] `esm-subpath-smoke.test.ts` passes: all three sub-path imports
      resolve to functions.
- [ ] `js-ci.yml` size gate step runs and reports a size within budget
      on a branch with no binary additions.
- [ ] `scripts/check_version_consistency.sh` passes when `package.json`
      and `Cargo.toml` carry the same version; fails with a clear message
      when they differ.
- [ ] `check_version_consistency.sh` is called in `js-ci.yml` on PRs
      that touch `javascript/@tinyquant/core/**` or
      `rust/crates/tinyquant-js/**`.
- [ ] `scripts/check_no_exec.sh` exists and is called in `js-ci.yml`
      after the build step; it reports PASS on the current `dist/` output.
- [ ] musl matrix cells (`x86_64-unknown-linux-musl`,
      `aarch64-unknown-linux-musl`) added to `js-ci.yml` and the
      resulting `.node` binaries included in the npm tarball.
- [ ] `loader.test.ts` musl key tests pass.
- [ ] All 7 JS gaps updated to `Gap: None.` in
      `docs/requirements/js.md` and the gap summary table in
      `docs/requirements/testing-gaps.md`.

## Risks

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| `corpus.setCompressionPolicy` not exposed through N-API | Medium | Low | Confirm API surface from Phase 25; if method is absent, test immutability by inspecting the JS object's property descriptor instead |
| aarch64-musl cross-linker setup is fragile in GitHub Actions | Medium | Medium | Pin `gcc-aarch64-linux-gnu` version; use a well-tested musl-cross Docker action as fallback |
| `npm pack --dry-run` output format changes across npm versions | Low | Low | Parse both `package size` and `Packaged size` variants; add a fallback grep |
| ESM sub-path imports fail in Jest's CJS transform mode | Low | Medium | Use `jest --experimental-vm-modules` or the Bun test runner for the ESM smoke test; add a comment explaining the runner requirement |
| musl binaries push gzip total above the 10 MB size gate | Low | Low | Two musl `.node` files add ~3 MB; current budget is ~4 MB → ~7 MB total, well within the 10 MB gate |

## See also

- [[requirements/testing-gaps|Testing Gaps]] — canonical gap descriptions
- [[requirements/js|JS/TS Requirements]] — FR-JS-002, FR-JS-004, FR-JS-006–010
- [[plans/rust/phase-25-typescript-npm-package|Phase 25]] — existing JS package structure and loader design
- [[plans/rust/phase-26-preparedcodec-calibration|Phase 26]] — parallel Rust calibration gate restoration
- [[plans/rust/phase-27-gpu-wgpu|Phase 27]] — musl-clean build also required here (coordinate matrix)
