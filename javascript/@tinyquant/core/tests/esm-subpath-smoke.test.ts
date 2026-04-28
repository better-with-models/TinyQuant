// tests/esm-subpath-smoke.test.ts
//
// GAP-JS-007: verify that each sub-path export resolves and exposes
// the expected constructor.
//
// Why relative imports instead of "tinyquant/codec"?
// The test suite runs from source (npm test = build + tsc + node --test)
// before the package is published to a registry. The package is not
// self-installed in node_modules, so bare sub-path specifiers would cause
// ERR_MODULE_NOT_FOUND. Instead we import from the same dist/ paths that
// the package.json "exports" field points to; a broken "exports" entry
// would produce a file-not-found or missing-export error at import time
// and fail the test. This is equivalent to testing the sub-path contracts.
//
// The tests are deliberately shallow: each checks that the named export is
// a function (JS class constructor), not undefined or a plain object. Full
// functional tests are in the type-specific test files.

import { describe, it } from "node:test";
import assert from "node:assert/strict";

// Sub-path: ./codec → dist/codec.js
import { CodecConfig } from "../dist/codec.js";

// Sub-path: ./corpus → dist/corpus.js
import { Corpus } from "../dist/corpus.js";

// Sub-path: ./backend → dist/backend.js
import { BruteForceBackend } from "../dist/backend.js";

describe("tinyquant — sub-path exports (GAP-JS-007)", () => {
  it("dist/codec.js exports CodecConfig as a constructor", () => {
    assert.equal(
      typeof CodecConfig,
      "function",
      "CodecConfig must be a constructor (function), not undefined or object",
    );
  });

  it("dist/corpus.js exports Corpus as a constructor", () => {
    assert.equal(
      typeof Corpus,
      "function",
      "Corpus must be a constructor (function), not undefined or object",
    );
  });

  it("dist/backend.js exports BruteForceBackend as a constructor", () => {
    assert.equal(
      typeof BruteForceBackend,
      "function",
      "BruteForceBackend must be a constructor (function), not undefined or object",
    );
  });
});
