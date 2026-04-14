//! `tinyquant corpus ingest` — write an FP32 matrix to a corpus file
//! under a chosen [`crate::IngestPolicy`].
//!
//! ## Policy dispatch
//!
//! - `compress` → requires `--codebook` + `--config-json`, drives the
//!   codec pipeline exactly like `codec compress`, and writes a TQCV
//!   file.
//! - `passthrough` / `fp16` → not yet covered by the CLI surface
//!   because `tinyquant-io::codec_file` currently only emits
//!   codec-compressed records. These branches return an explicit
//!   "not yet supported" error tagged [`CliErrorKind::InvalidArgs`]
//!   so operators get a clear signal, and Phase 23+ can fold in the
//!   other policies without breaking the CLI contract.

use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};

use crate::commands::codec_compress;
use crate::commands::CliErrorKind;
use crate::{IngestPolicy, VectorFormat};

/// Arguments for `corpus ingest`.
#[derive(Debug)]
pub struct Args {
    /// Input matrix path.
    pub input: PathBuf,
    /// Number of vectors (required with `--format f32`).
    pub rows: Option<usize>,
    /// Dimension per vector (required with `--format f32`).
    pub cols: Option<usize>,
    /// Config sidecar (required for `compress`).
    pub config_json: Option<PathBuf>,
    /// Codebook (required for `compress`).
    pub codebook: Option<PathBuf>,
    /// Output corpus path.
    pub corpus_path: PathBuf,
    /// Policy.
    pub policy: IngestPolicy,
    /// Thread count.
    pub threads: usize,
    /// Input matrix format.
    pub format: VectorFormat,
    /// Suppress `indicatif` progress bar (propagated from the global
    /// `--no-progress` flag).
    pub no_progress: bool,
}

/// Run `tinyquant corpus ingest`.
///
/// # Errors
///
/// Returns [`anyhow::Error`] wrapping a [`CliErrorKind`] for argument,
/// I/O, or policy failures.
pub fn run(args: Args) -> Result<()> {
    match args.policy {
        IngestPolicy::Compress => {
            let config_json = args.config_json.ok_or_else(|| {
                anyhow!("--config-json is required for --policy compress")
                    .context(CliErrorKind::InvalidArgs)
            })?;
            let codebook = args.codebook.ok_or_else(|| {
                anyhow!("--codebook is required for --policy compress")
                    .context(CliErrorKind::InvalidArgs)
            })?;
            codec_compress::run(codec_compress::Args {
                input: args.input,
                rows: args.rows,
                cols: args.cols,
                config_json,
                codebook,
                output: args.corpus_path,
                threads: args.threads,
                format: args.format,
                no_progress: args.no_progress,
            })
        }
        IngestPolicy::Passthrough | IngestPolicy::Fp16 => Err(anyhow!(
            "--policy {:?} is not yet supported by the standalone CLI — \
             the codec-file writer currently only emits compressed records. \
             Use `codec compress` for the `compress` policy.",
            args.policy
        ))
        .context(CliErrorKind::InvalidArgs),
    }
}
