// tests/round-trip.test.ts
//
// End-to-end round-trip fidelity for the N-API codec binding.
//
// Test cases:
//   (a) N=10 000, dim=128 — baseline codec fidelity gate (Phase 25).
//   (b) N=1 000,  dim=768 — GAP-JS-002: higher-dim regression guard.
//       A bug in the N-API Float32Array bridge for large dims would be
//       invisible with dim=128 alone. dim=768 is the smallest typical
//       embedding dimension where such bugs appear.
//
// Why hand-rolled PRNGs: pulling a PRNG dependency into a public package
// for a single test adds transitive install cost with no benefit.
// mulberry32 (dim=128 test) and xorshift32 (dim=768 test) both give
// adequate entropy for codec fidelity validation.

import { describe, it } from "node:test";
import assert from "node:assert/strict";

// See parity.test.ts for the rationale on importing from `../dist/`.
import { Codec, CodecConfig, Codebook } from "../dist/index.js";

// GAP-JS-002: deterministic xorshift32 generator for dim=768 test.
// Returns a Float32Array of `dim` values in [-1, 1].
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

// Element-wise mean squared error between two Float32Arrays.
function mse(a: Float32Array, b: Float32Array): number {
  let sum = 0;
  for (let i = 0; i < a.length; i++) sum += (a[i]! - b[i]!) ** 2;
  return sum / a.length;
}

/**
 * Deterministic PRNG (mulberry32). Returns a float in `[0, 1)`.
 * The initial state is the seed; repeated calls advance it.
 */
function makeRng(seed: number): () => number {
  let state = seed >>> 0;
  return () => {
    state = (state + 0x6d2b79f5) >>> 0;
    let t = state;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

function uniformVector(rng: () => number, dim: number): Float32Array {
  const v = new Float32Array(dim);
  for (let i = 0; i < dim; i++) {
    // Uniform [-1, 1)
    v[i] = rng() * 2 - 1;
  }
  return v;
}

describe("@better-with-models/tinyquant-core — round-trip", () => {
  const N = 10_000;
  const CAL = 1_000;
  const DIM = 128;
  const BITS = 4;

  it(`compress → decompress with bits=${BITS}, dim=${DIM}`, () => {
    // arbitrary but fixed so any future MSE regression reproduces
    const rng = makeRng(0xcafebabe);

    // Pre-generate all vectors so training and round-trip see the
    // same underlying numbers (the codebook needs representative
    // statistics; using the first CAL rows of the same distribution
    // matches what downstream users will typically do).
    const vectors: Float32Array[] = new Array(N);
    for (let i = 0; i < N; i++) vectors[i] = uniformVector(rng, DIM);

    // Flatten calibration subset for the train() call.
    const calibration = new Float32Array(CAL * DIM);
    for (let i = 0; i < CAL; i++) calibration.set(vectors[i]!, i * DIM);

    const cfg = new CodecConfig({
      bitWidth: BITS,
      seed: 0xdeadn,
      dimension: DIM,
      residualEnabled: false,
    });

    const codebook = Codebook.train(calibration, cfg);
    const codec = new Codec();

    let sumSq = 0;
    let count = 0;
    for (let i = 0; i < N; i++) {
      const original = vectors[i]!;
      const cv = codec.compress(original, cfg, codebook);
      const decoded = codec.decompress(cv, cfg, codebook);
      assert.equal(decoded.length, DIM);
      for (let j = 0; j < DIM; j++) {
        const d = original[j]! - decoded[j]!;
        sumSq += d * d;
        count++;
      }
    }

    const meanMse = sumSq / count;
    assert.ok(
      meanMse < 1e-2,
      `expected MSE < 1e-2, got ${meanMse.toExponential(3)} over ${count} scalars`,
    );
  });

  // GAP-JS-002: dim=768 round-trip. Uses N=1000 (not 10k) to keep the
  // test under 5 s. Codebook trained on 256 vectors; the remaining 744
  // are test vectors. A regression at the N-API boundary for higher-dim
  // inputs would be invisible without this case.
  it("round-trips N=1000, dim=768 vectors with MSE < 1e-2", () => {
    const DIM = 768;
    const N = 1_000;
    const NTRAIN = 256;
    const SEED = 0xdeadbeef;

    const cfg = new CodecConfig({
      bitWidth: 4,
      seed: BigInt(SEED),
      dimension: DIM,
      residualEnabled: false,
    });

    // Flatten 256 training vectors into one Float32Array for Codebook.train.
    const calibration = new Float32Array(NTRAIN * DIM);
    for (let i = 0; i < NTRAIN; i++) {
      calibration.set(seededVector(DIM, SEED ^ i), i * DIM);
    }
    const codebook = Codebook.train(calibration, cfg);
    const codec = new Codec();

    let totalMse = 0;
    for (let i = 0; i < N; i++) {
      const v = seededVector(DIM, SEED ^ (i + NTRAIN));
      const cv = codec.compress(v, cfg, codebook);
      const decompressed = codec.decompress(cv, cfg, codebook);
      assert.equal(decompressed.length, DIM);
      totalMse += mse(v, decompressed);
    }
    const meanMse = totalMse / N;

    assert.ok(
      meanMse < 1e-2,
      `mean MSE for dim=768 round-trip was ${meanMse.toFixed(6)}, expected < 1e-2`,
    );
  });
});
