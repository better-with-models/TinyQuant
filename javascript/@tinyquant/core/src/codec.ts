// src/codec.ts
//
// Thin wrapper module re-exporting the codec value objects from the
// native binding. Hand-written JSDoc lives here so consumers see
// documentation on hover without having to cross-reference the
// Python fat wheel or the Rust crate. All behaviour is delegated to
// the underlying `#[napi]` classes in `rust/crates/tinyquant-js`.
import { native } from "./_loader.js";
import type {
  CodecConfigOpts,
  NativeCodebook,
  NativeCodec,
  NativeCodecConfig,
  NativeCompressedVector,
  NativeRotationMatrix,
} from "./_loader.js";

export type { CodecConfigOpts };

/**
 * 64-hex SHA-256 digest identifying a codec configuration.
 *
 * Byte-identical across Python, Rust, and TypeScript implementations —
 * the canonical string format is documented in
 * `rust/crates/tinyquant-core/src/codec/codec_config.rs`.
 */
export type ConfigHash = string;

/**
 * Immutable value object describing codec parameters (bit width,
 * seed, dimension, residual flag). `seed` is a `bigint` because u64
 * values cannot round-trip through JS `number` losslessly.
 */
export const CodecConfig: new (opts: CodecConfigOpts) => NativeCodecConfig =
  native.CodecConfig;
export type CodecConfig = NativeCodecConfig;

/**
 * Immutable lookup table mapping quantised indices to FP32 values.
 * Construct via the static {@link train} factory — the native
 * constructor is private.
 */
export const Codebook: { train(vectors: Float32Array, config: NativeCodecConfig): NativeCodebook } =
  native.Codebook;
export type Codebook = NativeCodebook;

/**
 * Immutable output of a codec compression pass. Produced by
 * {@link Codec.compress} and the module-level {@link compress}. Not
 * directly constructable from JS.
 */
export const CompressedVector: new () => NativeCompressedVector =
  native.CompressedVector;
export type CompressedVector = NativeCompressedVector;

/**
 * Deterministic orthogonal rotation matrix for vector preconditioning.
 * Build via {@link fromConfig}; the rotation is fully determined by
 * the codec config's seed and dimension.
 */
export const RotationMatrix: { fromConfig(config: NativeCodecConfig): NativeRotationMatrix } =
  native.RotationMatrix;
export type RotationMatrix = NativeRotationMatrix;

/**
 * Stateless codec service. Instances carry no mutable state — each
 * `compress` / `decompress` call is independent. Equivalent to the
 * module-level {@link compress} / {@link decompress} convenience fns.
 */
export const Codec: new () => NativeCodec = native.Codec;
export type Codec = NativeCodec;

/**
 * Module-level convenience: equivalent to `new Codec().compress(...)`.
 * Prefer this over constructing a `Codec` when you only need a one-shot
 * call — the native side is already stateless.
 */
export const compress: (
  vector: Float32Array,
  config: NativeCodecConfig,
  codebook: NativeCodebook,
) => NativeCompressedVector = native.compress;

/**
 * Module-level convenience: equivalent to `new Codec().decompress(...)`.
 */
export const decompress: (
  compressed: NativeCompressedVector,
  config: NativeCodecConfig,
  codebook: NativeCodebook,
) => Float32Array = native.decompress;
