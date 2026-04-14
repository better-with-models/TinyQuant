//! Dispatch module for `tinyquant codec ...`.
//!
//! Maps the [`CodecCmd`] enum variants onto their implementation
//! modules. Each arm is kept deliberately thin so the type-level match
//! stays exhaustive under future additions.

use anyhow::Result;

use crate::CodecCmd;

use super::{codec_compress, codec_decompress, codec_train};

/// Dispatch a parsed `codec` subcommand.
///
/// # Errors
///
/// Propagates errors from the underlying `run` function.
pub fn dispatch(cmd: CodecCmd, no_progress: bool) -> Result<()> {
    match cmd {
        CodecCmd::Train {
            input,
            rows,
            cols,
            bit_width,
            seed,
            residual,
            format,
            output,
            config_out,
        } => codec_train::run(codec_train::Args {
            input,
            rows,
            cols,
            bit_width,
            seed,
            residual,
            format,
            output,
            config_out,
        }),
        CodecCmd::Compress {
            input,
            rows,
            cols,
            config_json,
            codebook,
            output,
            threads,
            format,
        } => codec_compress::run(codec_compress::Args {
            input,
            rows,
            cols,
            config_json,
            codebook,
            output,
            threads,
            format,
            no_progress,
        }),
        CodecCmd::Decompress {
            input,
            config_json,
            codebook,
            output,
            threads,
            format,
        } => codec_decompress::run(codec_decompress::Args {
            input,
            config_json,
            codebook,
            output,
            threads,
            format,
            no_progress,
        }),
    }
}
