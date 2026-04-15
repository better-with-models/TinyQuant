// tests/corpus.test.ts
//
// Fixture-driven corpus parity: for every scenario emitted by
// `scripts/packaging/generate_js_parity_fixtures.py`, construct the
// same corpus in TS and assert the vector count, contained ids, and
// event types match the Python oracle.
//
// The underlying numbers come from the Rust core (the Python wheel
// is the Rust-backed fat wheel via the Phase 24.1 shim), so this
// test also functions as a self-parity check for the napi-rs binding.

import { describe, it } from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

import {
  Codebook,
  CodecConfig,
  CompressionPolicy,
  Corpus,
  type CompressionPolicyKind,
} from "../dist/index.js";

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

interface CorpusScenario {
  scenario_id: string;
  config: {
    bit_width: number;
    seed: string;
    dimension: number;
    residual_enabled: boolean;
  };
  policy: CompressionPolicyKind;
  vectors: Record<string, number[]>;
  use_batch?: boolean;
  expected_events: string[];
  expected_vector_count: number;
  expected_contains: string[];
}

function loadScenarios(): CorpusScenario[] {
  const root = findPackageRoot(HERE);
  const file = path.join(root, "tests/fixtures/parity/corpus_scenarios.json");
  const raw = fs.readFileSync(file, "utf8");
  return JSON.parse(raw) as CorpusScenario[];
}

function policyFromTag(tag: CompressionPolicyKind): CompressionPolicy {
  switch (tag) {
    case "compress":
      return CompressionPolicy.COMPRESS;
    case "passthrough":
      return CompressionPolicy.PASSTHROUGH;
    case "fp16":
      return CompressionPolicy.FP16;
  }
}

// Training data needs to be deterministic and match the Python
// fixture's expectations; the scenarios only require the vector
// count / event types to match, not exact codebook entries, so any
// reasonable calibration works here.
function deterministicCalibration(dim: number, rows: number, seed: number): Float32Array {
  let state = seed >>> 0;
  const out = new Float32Array(rows * dim);
  for (let i = 0; i < out.length; i++) {
    state = (state + 0x6d2b79f5) >>> 0;
    let t = state;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    out[i] = ((t ^ (t >>> 14)) >>> 0) / 4294967296 - 0.5;
  }
  return out;
}

describe("@better-with-models/tinyquant-core — corpus scenarios from Python oracle", () => {
  const scenarios = loadScenarios();

  for (const scenario of scenarios) {
    it(`scenario: ${scenario.scenario_id}`, () => {
      const cfg = new CodecConfig({
        bitWidth: scenario.config.bit_width,
        seed: BigInt(scenario.config.seed),
        dimension: scenario.config.dimension,
        residualEnabled: scenario.config.residual_enabled,
      });
      const calibration = deterministicCalibration(cfg.dimension, 512, 0xc0ffee);
      const codebook = Codebook.train(calibration, cfg);
      const corpus = new Corpus({
        corpusId: "fixtures",
        codecConfig: cfg,
        codebook,
        compressionPolicy: policyFromTag(scenario.policy),
      });

      const entries = Object.entries(scenario.vectors);

      if (scenario.use_batch) {
        const batch: Record<string, Float32Array> = {};
        for (const [id, values] of entries) batch[id] = new Float32Array(values);
        corpus.insertBatch(batch);
      } else {
        for (const [id, values] of entries) {
          corpus.insert(id, new Float32Array(values));
        }
      }

      assert.equal(corpus.vectorCount, scenario.expected_vector_count);
      for (const id of scenario.expected_contains) {
        assert.ok(corpus.contains(id), `corpus should contain ${id}`);
        assert.ok(corpus.vectorIds.has(id), `vectorIds should contain ${id}`);
      }

      const events = corpus.pendingEvents();
      const eventTypes = events.map((e) => e.type);
      assert.deepEqual(eventTypes, scenario.expected_events);

      for (const event of events) {
        assert.ok(event.timestamp instanceof Date, "event.timestamp must be a Date");
      }
    });
  }

  it("round-trip: decompressAll emits a CorpusDecompressed event", () => {
    const cfg = new CodecConfig({
      bitWidth: 4,
      seed: 0xfeedn,
      dimension: 8,
      residualEnabled: false,
    });
    const calibration = deterministicCalibration(8, 256, 0x1234);
    const codebook = Codebook.train(calibration, cfg);
    const corpus = new Corpus({
      corpusId: "rt",
      codecConfig: cfg,
      codebook,
      compressionPolicy: CompressionPolicy.COMPRESS,
    });
    corpus.insert("a", new Float32Array([1, 0, 0, 0, 0, 0, 0, 0]));
    corpus.insert("b", new Float32Array([0, 1, 0, 0, 0, 0, 0, 0]));
    corpus.pendingEvents(); // drain CorpusCreated + VectorsInserted

    const all = corpus.decompressAll();
    assert.ok("a" in all, "decompressAll must include id 'a'");
    assert.ok("b" in all, "decompressAll must include id 'b'");
    assert.equal(all["a"]!.length, 8);

    const events = corpus.pendingEvents();
    assert.equal(events.length, 1);
    assert.equal(events[0]!.type, "CorpusDecompressed");
  });

  it("remove returns true for known id, false for unknown", () => {
    const cfg = new CodecConfig({
      bitWidth: 4,
      seed: 1n,
      dimension: 4,
      residualEnabled: false,
    });
    const calibration = deterministicCalibration(4, 128, 7);
    const codebook = Codebook.train(calibration, cfg);
    const corpus = new Corpus({
      corpusId: "rm",
      codecConfig: cfg,
      codebook,
      compressionPolicy: CompressionPolicy.COMPRESS,
    });
    corpus.insert("x", new Float32Array([1, 1, 1, 1]));
    assert.equal(corpus.remove("x"), true);
    assert.equal(corpus.remove("x"), false);
    assert.equal(corpus.vectorCount, 0);
  });
});
