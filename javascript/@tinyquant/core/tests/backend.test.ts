// tests/backend.test.ts
//
// Fixture-driven parity for the `BruteForceBackend` napi-rs binding.
// For each scenario in `tests/fixtures/parity/backend_scenarios.json`
// we ingest the same 10 vectors into the TS-side backend, replay the
// recorded queries, and assert that both the top-k ordering (by
// `vectorId`) and the cosine-similarity scores (within 1e-6) match
// the Python oracle byte-for-byte.
//
// The Python oracle runs against the Rust-backed fat wheel via the
// Phase 24.1 shim, so this also serves as a self-parity guard on the
// napi binding.

import { describe, it } from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { BruteForceBackend, type SearchResult } from "../dist/index.js";

const HERE = path.dirname(fileURLToPath(import.meta.url));

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

interface BackendQuery {
  query: number[];
  top_k: number;
  expected: Array<{ vector_id: string; score: number }>;
}

interface BackendScenario {
  scenario_id: string;
  dimension: number;
  vectors: Record<string, number[]>;
  queries: BackendQuery[];
}

function loadScenarios(): BackendScenario[] {
  const root = findPackageRoot(HERE);
  const file = path.join(root, "tests/fixtures/parity/backend_scenarios.json");
  const raw = fs.readFileSync(file, "utf8");
  return JSON.parse(raw) as BackendScenario[];
}

const SCORE_TOL = 1e-6;

describe("@better-with-models/tinyquant-core — BruteForceBackend parity", () => {
  const scenarios = loadScenarios();

  for (const scenario of scenarios) {
    it(`scenario: ${scenario.scenario_id}`, () => {
      const backend = new BruteForceBackend();

      const ingestMap: Record<string, Float32Array> = {};
      for (const [id, values] of Object.entries(scenario.vectors)) {
        ingestMap[id] = new Float32Array(values);
      }
      backend.ingest(ingestMap);
      assert.equal(backend.count, Object.keys(scenario.vectors).length);

      for (const q of scenario.queries) {
        const results: SearchResult[] = backend.search(
          new Float32Array(q.query),
          q.top_k,
        );
        assert.equal(
          results.length,
          q.expected.length,
          "top-k length must match Python oracle",
        );
        for (let i = 0; i < results.length; i++) {
          assert.equal(
            results[i]!.vectorId,
            q.expected[i]!.vector_id,
            `rank ${i}: vectorId mismatch`,
          );
          const delta = Math.abs(results[i]!.score - q.expected[i]!.score);
          assert.ok(
            delta < SCORE_TOL,
            `rank ${i}: score ${results[i]!.score} vs expected ${q.expected[i]!.score} (delta ${delta})`,
          );
        }
      }
    });
  }

  it("clear() resets count to zero and permits fresh ingest", () => {
    const backend = new BruteForceBackend();
    backend.ingest({
      a: new Float32Array([1, 0, 0, 0]),
      b: new Float32Array([0, 1, 0, 0]),
    });
    assert.equal(backend.count, 2);
    backend.clear();
    assert.equal(backend.count, 0);
    backend.ingest({ c: new Float32Array([0, 0, 1, 0]) });
    assert.equal(backend.count, 1);
  });

  it("remove drops listed ids and leaves the rest intact", () => {
    const backend = new BruteForceBackend();
    backend.ingest({
      a: new Float32Array([1, 0, 0, 0]),
      b: new Float32Array([0, 1, 0, 0]),
      c: new Float32Array([0, 0, 1, 0]),
    });
    backend.remove(["b"]);
    assert.equal(backend.count, 2);
    const hits = backend.search(new Float32Array([1, 0, 0, 0]), 3);
    const ids = hits.map((h) => h.vectorId).sort();
    assert.deepEqual(ids, ["a", "c"]);
  });
});
