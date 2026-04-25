// tests/corpus.test.ts
//
// Two describe blocks:
//
//   (1) "corpus scenarios from Python oracle" — fixture-driven parity for
//       every scenario emitted by scripts/packaging/generate_js_parity_fixtures.py.
//       Asserts vector count, contained ids, and event types match the Python
//       oracle. Because the Python wheel is backed by the same Rust core (Phase 24.1
//       shim), this also functions as a self-parity check for the napi-rs binding.
//
//   (2) "corpus policy invariants (GAP-JS-004)" — closes the gap identified in
//       testing-gaps.md §GAP-JS-004: config-hash distinctness, compressionPolicy
//       immutability, and FP16 round-trip precision through the N-API boundary.

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

// GAP-JS-004: xorshift32 deterministic vector for policy invariant tests.
// Returns Float32Array of `dim` values in [-1, 1].
function seededVector(dim: number, seed: number): Float32Array {
  let s = seed >>> 0;
  const out = new Float32Array(dim);
  for (let i = 0; i < dim; i++) {
    s ^= s << 13;
    s ^= s >> 17;
    s ^= s << 5;
    out[i] = ((s >>> 0) / 0xffffffff) * 2 - 1;
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

// ---------------------------------------------------------------------------
// GAP-JS-004 — Corpus policy invariants exercised through the N-API boundary.
//
// These three cases close the gap identified in testing-gaps.md §GAP-JS-004:
// cross-config hash distinctness, policy stability, and FP16 round-trip
// precision. They run in a separate describe block so CI output is easy to
// grep.
//
// Note on cross-config rejection: the full rejection test requires
// `corpus.insertCompressed()`, which is not yet exposed by the N-API binding.
// The test below verifies the config-hash mechanism that underpins the check
// (distinct seeds → distinct hashes; VectorEntry hash matches corpus hash).
// ---------------------------------------------------------------------------
describe("@better-with-models/tinyquant-core — corpus policy invariants (GAP-JS-004)", () => {
  const DIM = 64;
  const NTRAIN = 256;

  // GAP-JS-004 (1/3): Two corpora with different seeds produce distinct config
  // hashes, and the hash recorded on a VectorEntry matches the corpus config.
  // This exercises the config-hash path through the N-API boundary and lays the
  // groundwork for cross-config rejection (which requires insertCompressed).
  it("config hash differs between seeds and matches VectorEntry through N-API", () => {
    const cfgA = new CodecConfig({ bitWidth: 4, seed: 1n, dimension: DIM, residualEnabled: false });
    const cfgB = new CodecConfig({ bitWidth: 4, seed: 2n, dimension: DIM, residualEnabled: false });

    assert.notEqual(
      cfgA.configHash,
      cfgB.configHash,
      "configs with different seeds must have distinct hashes",
    );

    const calA = deterministicCalibration(DIM, NTRAIN, 1);
    const cbA = Codebook.train(calA, cfgA);
    const corpusA = new Corpus({
      corpusId: "gap-004-a",
      codecConfig: cfgA,
      codebook: cbA,
      compressionPolicy: CompressionPolicy.COMPRESS,
    });
    const entry = corpusA.insert("v0", seededVector(DIM, 10));

    assert.equal(
      entry.configHash,
      cfgA.configHash,
      "VectorEntry.configHash must equal the corpus config hash through N-API",
    );
    assert.notEqual(
      entry.configHash,
      cfgB.configHash,
      "VectorEntry.configHash must not match a different config",
    );
  });

  // GAP-JS-004 (2/3): compressionPolicy is immutable after insertion.
  //
  // The N-API binding does not expose a setCompressionPolicy() mutation method.
  // We verify immutability in two ways:
  //   (a) accessor methods return correct values after insertion (smoke),
  //   (b) direct property assignment does not change the policy value
  //       (guards against an accidental writable property on the binding object).
  it("compressionPolicy is immutable after insertion through N-API", () => {
    const cfg = new CodecConfig({ bitWidth: 4, seed: 42n, dimension: DIM, residualEnabled: false });
    const cal = deterministicCalibration(DIM, NTRAIN, 42);
    const cb = Codebook.train(cal, cfg);
    const corpus = new Corpus({
      corpusId: "gap-004-pi",
      codecConfig: cfg,
      codebook: cb,
      compressionPolicy: CompressionPolicy.COMPRESS,
    });
    corpus.insert("v0", seededVector(DIM, 0));
    corpus.insert("v1", seededVector(DIM, 1));

    // (a) policy accessor methods return expected values.
    assert.equal(corpus.compressionPolicy.kind, "compress");
    // requiresCodec() must return true for the COMPRESS policy.
    assert.equal(corpus.compressionPolicy.requiresCodec(), true);
    // storageDtype() must be uint8 (codec path).
    assert.equal(corpus.compressionPolicy.storageDtype(), "uint8");

    // (b) direct property write must not stick.
    // In strict mode a sealed/frozen property throws TypeError; in sloppy mode
    // the assignment is silently ignored. Either outcome is acceptable — the
    // invariant is that the policy does not change.
    try {
      (corpus as unknown as Record<string, unknown>)["compressionPolicy"] =
        CompressionPolicy.PASSTHROUGH;
    } catch {
      // strict-mode TypeError from a non-writable property: expected and acceptable.
    }
    assert.equal(
      corpus.compressionPolicy.kind,
      "compress",
      "compressionPolicy must not change via direct property write",
    );
  });

  // GAP-JS-004 (3/3): FP16 policy round-trip precision through N-API.
  // Each decompressed element must be within 2^-10 × |original| of the
  // original (IEEE 754 half-precision machine epsilon). This is looser than
  // the 2^-13 bound referenced in the plan to avoid false failures at the
  // FP16 rounding boundary for arbitrary xorshift32 values in [-1, 1].
  it("FP16 policy preserves each element within FP16 precision through N-API", () => {
    const cfg = new CodecConfig({ bitWidth: 4, seed: 42n, dimension: DIM, residualEnabled: false });
    const cal = deterministicCalibration(DIM, NTRAIN, 42);
    const cb = Codebook.train(cal, cfg);
    const corpus = new Corpus({
      corpusId: "gap-004-fp16",
      codecConfig: cfg,
      codebook: cb,
      compressionPolicy: CompressionPolicy.FP16,
    });

    const original = seededVector(DIM, 7);
    corpus.insert("v0", original);
    const all = corpus.decompressAll();

    assert.ok("v0" in all, "decompressAll must contain the inserted id");
    const decompressed = all["v0"]!;
    assert.equal(decompressed.length, DIM);

    // FP16 machine epsilon is 2^-10; allow one ULP of headroom.
    const REL_BOUND = Math.pow(2, -10);
    const ABS_FLOOR = 1e-6; // handles near-zero elements

    for (let i = 0; i < DIM; i++) {
      const o = original[i]!;
      const r = decompressed[i]!;
      const allowed = Math.abs(o) * REL_BOUND + ABS_FLOOR;
      assert.ok(
        Math.abs(o - r) <= allowed,
        `element ${i}: |${o.toFixed(6)} - ${r.toFixed(6)}| = ${Math.abs(o - r).toExponential(3)} exceeds FP16 bound ${allowed.toExponential(3)}`,
      );
    }
  });
});
