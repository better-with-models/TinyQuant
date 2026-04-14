// tests/types.test.ts
//
// Smoke test the public TypeScript surface of `@tinyquant/core`.
// The goal is not to exercise runtime behaviour (the other test
// files do that) but to catch surface drift: if a type export
// disappears, is renamed, or changes kind (class vs interface),
// `tsc -p tsconfig.test.json` will fail to compile this file and
// the build stops before `node --test` runs.
//
// Keep this file mechanical — one trivial assertion per exported
// symbol is enough.

import { describe, it } from "node:test";
import assert from "node:assert/strict";

import {
  Codec,
  Codebook,
  CodecConfig,
  CompressedVector,
  RotationMatrix,
  compress,
  decompress,
  CompressionPolicy,
  Corpus,
  VectorEntry,
  BruteForceBackend,
  SearchResult,
  TinyQuantError,
  version,
  type ConfigHash,
  type CompressionPolicyKind,
  type StorageDtype,
  type CorpusOpts,
  type CorpusEvent,
  type CorpusCreatedEvent,
  type VectorsInsertedEvent,
  type CorpusDecompressedEvent,
  type CompressionPolicyViolationDetectedEvent,
} from "../dist/index.js";

describe("@tinyquant/core — public surface smoke tests", () => {
  it("class handles are present and are constructors", () => {
    assert.equal(typeof CodecConfig, "function");
    assert.equal(typeof Codebook, "function");
    assert.equal(typeof CompressedVector, "function");
    assert.equal(typeof RotationMatrix, "function");
    assert.equal(typeof Codec, "function");
    assert.equal(typeof CompressionPolicy, "function");
    assert.equal(typeof Corpus, "function");
    assert.equal(typeof VectorEntry, "function");
    assert.equal(typeof BruteForceBackend, "function");
    assert.equal(typeof SearchResult, "function");
    assert.equal(typeof TinyQuantError, "function");
  });

  it("module-level functions are functions", () => {
    assert.equal(typeof compress, "function");
    assert.equal(typeof decompress, "function");
    assert.equal(typeof version, "function");
  });

  it("version() returns a non-empty semver-looking string", () => {
    const v = version();
    assert.equal(typeof v, "string");
    assert.ok(v.length > 0, "version string must be non-empty");
  });

  it("CompressionPolicy statics are policy instances", () => {
    assert.ok(CompressionPolicy.COMPRESS instanceof CompressionPolicy);
    assert.ok(CompressionPolicy.PASSTHROUGH instanceof CompressionPolicy);
    assert.ok(CompressionPolicy.FP16 instanceof CompressionPolicy);
    assert.equal(CompressionPolicy.COMPRESS.kind, "compress");
    assert.equal(CompressionPolicy.PASSTHROUGH.kind, "passthrough");
    assert.equal(CompressionPolicy.FP16.kind, "fp16");
    assert.equal(CompressionPolicy.COMPRESS.requiresCodec(), true);
    assert.equal(CompressionPolicy.PASSTHROUGH.requiresCodec(), false);
  });

  it("TinyQuantError.fromNative parses the class-name prefix", () => {
    const wrapped = TinyQuantError.fromNative(
      new Error("DimensionMismatchError: expected 8 got 4"),
    );
    assert.ok(wrapped instanceof TinyQuantError);
    assert.equal(wrapped.code, "DimensionMismatchError");
    assert.equal(wrapped.message, "expected 8 got 4");

    const unprefixed = TinyQuantError.fromNative(new Error("plain message"));
    assert.equal(unprefixed.code, "TinyQuantError");
    assert.equal(unprefixed.message, "plain message");
  });

  it("TinyQuantError.fromNative handles non-prefixed panic-shaped messages", () => {
    // Raw napi panics do not follow the `<ClassName>: reason` contract —
    // they start with `thread 'main' panicked: ...`. The prefix regex
    // rejects the leading `thread 'main' panicked` slice (contains a
    // space and a single quote) so we should fall back to the default
    // `code === "TinyQuantError"` and keep the original message intact.
    const raw = new Error("thread 'main' panicked: something broke");
    const wrapped = TinyQuantError.fromNative(raw);
    assert.ok(wrapped instanceof TinyQuantError);
    assert.equal(wrapped.code, "TinyQuantError");
    assert.match(wrapped.message, /thread 'main' panicked/);
    assert.equal(wrapped.cause, raw);
  });

  it("compile-time type shapes", () => {
    // Pure type assertions — body is trivial; failure is at tsc.
    const _policyKind: CompressionPolicyKind = "compress";
    const _dtype: StorageDtype = "uint8";
    const _hash: ConfigHash = "deadbeef";
    void _policyKind;
    void _dtype;
    void _hash;

    // Generic holder to prove the event discriminants are usable in
    // switch statements without `any` fallbacks.
    const renderEvent = (ev: CorpusEvent): string => {
      switch (ev.type) {
        case "CorpusCreated":
          return `created:${(ev as CorpusCreatedEvent).compressionPolicy}`;
        case "VectorsInserted":
          return `inserted:${(ev as VectorsInsertedEvent).count}`;
        case "CorpusDecompressed":
          return `decompressed:${(ev as CorpusDecompressedEvent).vectorCount}`;
        case "CompressionPolicyViolationDetected":
          return `violation:${(ev as CompressionPolicyViolationDetectedEvent).violationType}`;
      }
    };
    void renderEvent;

    // CorpusOpts must accept the minimum required shape.
    const optsShape: Pick<CorpusOpts, "corpusId"> = { corpusId: "x" };
    assert.equal(optsShape.corpusId, "x");
  });
});
