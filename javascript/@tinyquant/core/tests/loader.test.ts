// tests/loader.test.ts
//
// Unit tests for the binary-key computation in `src/_loader.ts`.
//
// Why import from dist/_loader.js directly?
// The loader calls loadNative() at module load time, which loads the
// .node binary for the current host platform. In the test-source CI job
// (ubuntu-22.04 / x64) the gnu binary is present after `npm run build`,
// so the import succeeds. The binaryKey() overrides used in each test
// only compute a string — they do not attempt to load a second binary.
//
// Coverage (FR-JS-006):
//   - All 5 glibc platform keys (linux-x64-gnu, linux-arm64-gnu,
//     darwin-x64, darwin-arm64, win32-x64-msvc)
//   - 2 musl platform keys (linux-x64-musl, linux-arm64-musl)
//   - Unsupported platform throws with platform/arch in message

import { describe, it } from "node:test";
import assert from "node:assert/strict";

import { binaryKey } from "../dist/_loader.js";

describe("@better-with-models/tinyquant-core — loader binary key (FR-JS-006)", () => {
  // -------------------------------------------------------------------
  // glibc platforms
  // -------------------------------------------------------------------
  it("returns 'linux-x64-gnu' for linux/x64/gnu", () => {
    assert.equal(binaryKey("linux", "x64", false), "linux-x64-gnu");
  });

  it("returns 'linux-arm64-gnu' for linux/arm64/gnu", () => {
    assert.equal(binaryKey("linux", "arm64", false), "linux-arm64-gnu");
  });

  it("returns 'darwin-x64' for darwin/x64", () => {
    assert.equal(binaryKey("darwin", "x64"), "darwin-x64");
  });

  it("returns 'darwin-arm64' for darwin/arm64", () => {
    assert.equal(binaryKey("darwin", "arm64"), "darwin-arm64");
  });

  it("returns 'win32-x64-msvc' for win32/x64", () => {
    assert.equal(binaryKey("win32", "x64"), "win32-x64-msvc");
  });

  // -------------------------------------------------------------------
  // GAP-JS-006: musl platform keys
  // -------------------------------------------------------------------
  it("computes 'linux-x64-musl' key for linux/x64 on musl", () => {
    assert.equal(binaryKey("linux", "x64", true), "linux-x64-musl");
  });

  it("computes 'linux-arm64-musl' key for linux/arm64 on musl", () => {
    assert.equal(binaryKey("linux", "arm64", true), "linux-arm64-musl");
  });

  // -------------------------------------------------------------------
  // Unsupported platform error
  // -------------------------------------------------------------------
  it("throws for an unsupported platform with platform/arch in the message", () => {
    assert.throws(
      () => binaryKey("freebsd", "x64"),
      (err: unknown) => {
        assert.ok(err instanceof Error, "must throw an Error");
        assert.ok(
          err.message.includes("freebsd") && err.message.includes("x64"),
          `error message must include platform/arch; got: ${err.message}`,
        );
        assert.ok(
          err.message.includes("github.com"),
          `error message must include a GitHub issues URL; got: ${err.message}`,
        );
        return true;
      },
    );
  });

  it("throws for unsupported arch on a known platform with platform/arch in message", () => {
    assert.throws(
      () => binaryKey("linux", "riscv64", false),
      (err: unknown) => {
        assert.ok(err instanceof Error, "must throw an Error");
        assert.ok(
          err.message.includes("linux") && err.message.includes("riscv64"),
          `error message must include platform/arch; got: ${(err as Error).message}`,
        );
        return true;
      },
    );
  });
});
