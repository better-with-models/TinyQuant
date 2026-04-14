// src/index.ts
//
// Phase 25.2 public surface: codec value objects + Codec + module-
// level compress/decompress. Corpus / backend land in later slices.
// The re-exports below are intentionally thin pass-throughs to the
// native binding; a user-facing TypeScript wrapper with JSDoc lands
// in Phase 25.4 (plan step 8).
import { native } from "./_loader.js";

export type {
  CodecConfigOpts,
  NativeCodecConfig,
  NativeCodebook,
  NativeCompressedVector,
  NativeRotationMatrix,
  NativeCodec,
} from "./_loader.js";

export const version: () => string = native.version;

export const CodecConfig = native.CodecConfig;
export const Codebook = native.Codebook;
export const CompressedVector = native.CompressedVector;
export const RotationMatrix = native.RotationMatrix;
export const Codec = native.Codec;

export const compress = native.compress;
export const decompress = native.decompress;
