//! `tinyquant codec compress` — compress an FP32 matrix into a TQCV
//! corpus file.
//!
//! Pipeline:
//!
//! 1. Read the config JSON sidecar + codebook emitted by `codec train`.
//! 2. Read the input matrix via [`crate::io::read_matrix`].
//! 3. Run [`tinyquant_core::codec::Codec::compress_batch_with`] with a
//!    rayon-driven [`tinyquant_core::codec::Parallelism::Custom`]
//!    honouring `--threads`.
//! 4. Stream the resulting [`tinyquant_core::codec::CompressedVector`]
//!    records into a [`tinyquant_io::codec_file::CodecFileWriter`].

use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use tinyquant_core::codec::{Codec, Parallelism};
use tinyquant_io::codec_file::CodecFileWriter;
use tracing::info;

use crate::commands::codebook_io::{read_codebook, read_config_json};
use crate::commands::CliErrorKind;
use crate::io::read_matrix;
use crate::progress;
use crate::VectorFormat;

/// Arguments for `codec compress`.
#[derive(Debug)]
pub struct Args {
    /// Input FP32 matrix path.
    pub input: PathBuf,
    /// Number of vectors (required with `--format f32`).
    pub rows: Option<usize>,
    /// Dimension per vector (required with `--format f32`).
    pub cols: Option<usize>,
    /// Config sidecar path.
    pub config_json: PathBuf,
    /// Codebook path.
    pub codebook: PathBuf,
    /// Output corpus path.
    pub output: PathBuf,
    /// Thread count.
    pub threads: usize,
    /// Input matrix format.
    pub format: VectorFormat,
    /// Suppress `indicatif` progress bar (propagated from the global
    /// `--no-progress` flag).
    pub no_progress: bool,
}

/// Run `tinyquant codec compress`.
///
/// # Errors
///
/// Returns [`anyhow::Error`] wrapping a [`CliErrorKind`] for argument,
/// I/O, or codec failures.
pub fn run(args: Args) -> Result<()> {
    let config = read_config_json(&args.config_json)?;
    let codebook = read_codebook(&args.codebook)?;

    if codebook.bit_width() != config.bit_width() {
        return Err(anyhow!(
            "codebook bit_width {} != config bit_width {}",
            codebook.bit_width(),
            config.bit_width()
        ))
        .context(CliErrorKind::Verify);
    }

    let (rows, cols, flat) = read_matrix(&args.input, args.format, args.rows, args.cols)?;

    let expected_cols = config.dimension() as usize;
    if cols != expected_cols {
        return Err(anyhow!(
            "input cols {cols} != config dimension {expected_cols}",
        ))
        .context(CliErrorKind::InvalidArgs);
    }

    validate_threads(args.threads)?;

    info!(
        rows,
        cols,
        threads = args.threads,
        path = %args.output.display(),
        "compressing batch"
    );

    let codec = Codec::new();
    // Phase 21 parity: drive the batch through a scoped rayon pool so
    // `--threads` is honoured without touching rayon's global pool.
    // The `Parallelism::Custom` driver is a bare fn pointer (cannot
    // capture the pool), hence the `pool.install(...)` pattern.
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(args.threads)
        .build()
        .context("building rayon thread pool")
        .map_err(|e| e.context(CliErrorKind::Other))?;

    // §Step 14: progress bar on long-running batches. The bar length
    // is the row count (one tick per per-row compression); tied to
    // the rayon chunk count is the chunk boundary only, which would
    // flash the user far too briefly. The driver ticks through the
    // globally-published handle in `crate::progress`; see that
    // module's docstring for why we go through a static.
    let compress_bar = progress::bar(
        u64::try_from(rows).unwrap_or(u64::MAX),
        "compressing",
        args.no_progress,
    );
    progress::set_active_compress_bar(Some(&compress_bar));

    let compressed = pool
        .install(|| {
            codec.compress_batch_with(
                &flat,
                rows,
                cols,
                &config,
                &codebook,
                Parallelism::Custom(rayon_driver),
            )
        })
        .map_err(|e| anyhow!("{e}"));
    progress::clear_active_compress_bar();
    compress_bar.finish();
    let compressed = compressed.map_err(|e| e.context(CliErrorKind::Other))?;

    let mut writer = CodecFileWriter::create(
        &args.output,
        config.config_hash().as_ref(),
        config.dimension(),
        config.bit_width(),
        config.residual_enabled(),
        &[],
    )
    .map_err(|e| anyhow!("{e}"))
    .map_err(|e| e.context(CliErrorKind::Io))?;

    // §Step 14: second bar for the serial writer path. Writer ticks
    // are coarse (one per record), so this is the right granularity.
    let write_bar = progress::bar(
        u64::try_from(compressed.len()).unwrap_or(u64::MAX),
        "writing",
        args.no_progress,
    );
    for cv in &compressed {
        writer
            .append(cv)
            .map_err(|e| anyhow!("{e}"))
            .map_err(|e| e.context(CliErrorKind::Io))?;
        write_bar.inc(1);
    }
    write_bar.finish();
    writer
        .finalize()
        .map_err(|e| anyhow!("{e}"))
        .map_err(|e| e.context(CliErrorKind::Io))?;

    info!(records = compressed.len(), "corpus finalized");
    Ok(())
}

/// Reject `--threads == 0`. Any positive value is fine; `threads == 1`
/// still drives through the rayon pool (it just builds a 1-worker
/// pool), which keeps the code path identical and the determinism
/// contract intact.
fn validate_threads(threads: usize) -> Result<()> {
    if threads == 0 {
        return Err(anyhow!("--threads must be >= 1")).context(CliErrorKind::InvalidArgs);
    }
    Ok(())
}

/// rayon-backed driver for `Parallelism::Custom`.
///
/// The `count` parameter is the number of rows; `body` is the
/// per-row worker callback. `rayon::scope` ensures every callback
/// has completed before returning, matching the determinism contract
/// documented in `tinyquant-core::codec::batch`.
///
/// After each row is compressed the driver ticks the active
/// `indicatif` progress bar installed by [`crate::progress`]. When no
/// bar is active (or the `progress` feature is off), the tick is a
/// cheap lock-free check of a `OnceLock`.
fn rayon_driver(count: usize, body: &(dyn Fn(usize) + Sync + Send)) {
    use rayon::prelude::*;
    (0..count).into_par_iter().for_each(|i| {
        body(i);
        crate::progress::tick_active_compress_bar();
    });
}
