// tests/parity.test.ts
//
// Cross-implementation parity: for every fixture case emitted by
// `scripts/packaging/generate_js_parity_fixtures.py`, the TS
// `CodecConfig.configHash` must match the Python reference's
// `config_hash` byte-for-byte.
//
// The Python reference and the Rust core share a single canonical
// string format (documented in
// `rust/crates/tinyquant-core/src/codec/codec_config.rs`), so the
// fixture's hashes are simultaneously the oracle for BOTH the
// Python shim and the Rust-backed TS binding.

import { describe, it } from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
const { readFileSync } = fs;
import path from "node:path";
import { fileURLToPath } from "node:url";

// Tests import the canonical published bundle — the same entry point
// external consumers resolve — so there is no second compiled copy
// of `src/` under `dist-tests/`. The main `build` script emits the
// `.d.ts` and `.js` files required for this import to type-check and
// load at runtime; the `test` script chains `build` before compiling
// the test sources.
import { CodecConfig } from "../dist/index.js";

const HERE = path.dirname(fileURLToPath(import.meta.url));

// When running from `dist-tests/tests/parity.test.js` the compiled
// test lives one level below the package root's compiled-tests dir.
// Fixtures are NOT copied into the build output — they live under
// `tests/fixtures/` in source. Walk up to the package root (the
// directory containing `package.json`) and then descend into
// `tests/fixtures`.
function findPackageRoot(start: string): string {
  let dir = start;
  for (let i = 0; i < 8; i++) {
    if (fs.existsSync(path.join(dir, "package.json"))) return dir;
    const parent = path.dirname(dir);
    if (parent === dir) break;
    dir = parent;
  }
  return start;
}

type ParityCase = {
  bit_width: number;
  // Seed arrives as a decimal string because u64 values >= 2^53 lose
  // precision through JSON number. The fixture generator always
  // stringifies it for uniformity.
  seed: string;
  dimension: number;
  residual_enabled: boolean;
  config_hash: string;
};

function loadFixtures(): ParityCase[] {
  const root = findPackageRoot(HERE);
  const raw = readFileSync(
    path.join(root, "tests", "fixtures", "parity", "config_hashes.json"),
    "utf8",
  );
  return JSON.parse(raw) as ParityCase[];
}

describe("@better-with-models/tinyquant-core — parity: config_hash", () => {
  const cases = loadFixtures();

  it("loaded at least 20 fixture cases", () => {
    assert.ok(cases.length >= 20, `expected >= 20 cases, got ${cases.length}`);
  });

  for (const c of cases) {
    it(`bit=${c.bit_width} seed=${c.seed} dim=${c.dimension} residual=${c.residual_enabled}`, () => {
      const cfg = new CodecConfig({
        bitWidth: c.bit_width,
        seed: BigInt(c.seed),
        dimension: c.dimension,
        residualEnabled: c.residual_enabled,
      });
      assert.equal(cfg.configHash, c.config_hash);
    });
  }
});
