// tests/cjs-smoke.test.cjs
//
// CommonJS consumer smoke test. `package.json` declares `"type":
// "module"`, so a real CJS bundle at `dist/index.cjs` is the only
// way `require("@tinyquant/core")` can work without throwing
// ERR_REQUIRE_ESM. This test exercises that exact require path, so
// a regression (e.g. `exports.require.default` accidentally pointing
// back at `dist/index.js`) lights up here.
const {
  version,
  CodecConfig,
  Corpus,
  BruteForceBackend,
  CompressionPolicy,
} = require("../dist/index.cjs");
const assert = require("node:assert/strict");
const { test } = require("node:test");

test("CJS require works", () => {
  assert.ok(typeof version() === "string");
  assert.ok(typeof CodecConfig === "function");
  assert.ok(typeof Corpus === "function");
  assert.ok(typeof BruteForceBackend === "function");
  assert.ok(typeof CompressionPolicy === "function");
  assert.ok(CompressionPolicy.COMPRESS);
});
