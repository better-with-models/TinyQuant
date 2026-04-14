// src/_errors.ts
//
// Error wrapper that parses the `"<ClassName>: reason"` message
// prefix produced by `rust/crates/tinyquant-js/src/errors.rs` and
// re-exposes the class name via `err.code`.
//
// napi-rs v2's `Status` enum does not admit custom string codes, so
// the Rust side encodes the Python-parity class name as a message
// prefix. Consumers who want to switch on error kind can either
// parse `err.message` themselves or wrap the call in
// `TinyQuantError.fromNative(err)`.

/**
 * Structured error type mirroring the Python fat wheel's exception
 * hierarchy. Construct from a native `Error` thrown across the FFI
 * via the {@link fromNative} factory; the `code` property carries
 * the Python-parity class name.
 *
 * @example
 * ```ts
 * import { Corpus, TinyQuantError } from "@tinyquant/core";
 * try {
 *   corpus.insert("dup", vector);
 * } catch (err) {
 *   const e = TinyQuantError.fromNative(err);
 *   if (e.code === "DuplicateVectorError") { ... }
 * }
 * ```
 */
export class TinyQuantError extends Error {
  /** Python-parity class name (e.g. `"DimensionMismatchError"`). */
  readonly code: string;
  /**
   * The original native error, preserved so consumers can inspect
   * `.stack` or other napi-rs `Status` fields if they need to.
   */
  readonly cause: unknown;

  constructor(code: string, message: string, cause?: unknown) {
    super(message);
    this.name = "TinyQuantError";
    this.code = code;
    this.cause = cause;
  }

  /**
   * Parse a native napi-rs error into a {@link TinyQuantError}. The
   * input may be any value — non-Error inputs are wrapped with
   * `code === "TinyQuantError"` so this never throws.
   *
   * Message prefix contract: `"<ClassName>: <reason>"`. The prefix
   * is stripped from `message`; the class name lands in `code`. If
   * no prefix is present, `code` defaults to `"TinyQuantError"`.
   */
  static fromNative(err: unknown): TinyQuantError {
    if (err instanceof TinyQuantError) return err;
    const raw = err instanceof Error ? err.message : String(err);
    const separator = raw.indexOf(": ");
    if (separator > 0 && /^[A-Za-z][A-Za-z0-9_]*$/.test(raw.slice(0, separator))) {
      const code = raw.slice(0, separator);
      const message = raw.slice(separator + 2);
      return new TinyQuantError(code, message, err);
    }
    return new TinyQuantError("TinyQuantError", raw, err);
  }
}
