//! GPU state attachment for `PreparedCodec`.
//!
//! Holds device-resident buffers (rotation matrix, codebook) uploaded
//! once via `WgpuBackend::prepare_for_device`.
//!
//! Full implementation follows in Part C (Phase 27 Steps 5-6).
