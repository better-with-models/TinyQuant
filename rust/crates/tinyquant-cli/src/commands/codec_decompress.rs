//! `tinyquant codec decompress` — stream every record in a TQCV corpus
//! back to raw FP32.
//!
//! Pipeline:
//!
//! 1. Read the config JSON sidecar + codebook.
//! 2. Open the input corpus file via
//!    [`tinyquant_io::codec_file::CodecFileReader`].
//! 3. For each record: `Codec::decompress_into` into a shared flat
//!    `Vec<f32>` (serial; the per-record decompress is fast and the
//!    I/O is sequential — parallel decompress would require reading
//!    the whole corpus upfront, at 4x the peak memory).
//! 4. Write the flat matrix via [`crate::io::write_matrix`].

use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use tinyquant_core::codec::Codec;
use tinyquant_io::codec_file::CodecFileReader;
use tracing::info;

use crate::commands::codebook_io::{read_codebook, read_config_json};
use crate::commands::CliErrorKind;
use crate::io::write_matrix;
use crate::progress;
use crate::VectorFormat;

/// Arguments for `codec decompress`.
#[derive(Debug)]
pub struct Args {
    /// Input corpus path.
    pub input: PathBuf,
    /// Config sidecar path.
    pub config_json: PathBuf,
    /// Codebook path.
    pub codebook: PathBuf,
    /// Output matrix path.
    pub output: PathBuf,
    /// Thread count (currently unused; the streaming decompress is
    /// sequential).
    pub threads: usize,
    /// Output matrix format.
    pub format: VectorFormat,
    /// Suppress `indicatif` progress bar (propagated from the global
    /// `--no-progress` flag).
    pub no_progress: bool,
}

/// Run `tinyquant codec decompress`.
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

    let file = File::open(&args.input)
        .with_context(|| format!("opening corpus {}", args.input.display()))
        .map_err(|e| e.context(CliErrorKind::Io))?;
    let mut reader = CodecFileReader::new(BufReader::new(file))
        .map_err(|e| anyhow!("{e}"))
        .map_err(|e| e.context(CliErrorKind::Verify))?;

    let header = reader.header();
    let vector_count = usize::try_from(header.vector_count)
        .map_err(|_| anyhow!("vector_count {} does not fit in usize", header.vector_count))
        .map_err(|e| e.context(CliErrorKind::InvalidArgs))?;
    let dim = header.dimension as usize;

    if header.dimension != config.dimension() {
        return Err(anyhow!(
            "corpus dimension {} != config dimension {}",
            header.dimension,
            config.dimension()
        ))
        .context(CliErrorKind::Verify);
    }

    let codec = Codec::new();
    let mut out: Vec<f32> = vec![0.0; vector_count * dim];
    info!(vector_count, dim, "decompressing corpus");
    // §Step 14: determinate bar across the streaming decompress.
    // `vector_count` is known up-front from the corpus header, so a
    // real bar (not a spinner) is appropriate here.
    let bar = progress::bar(
        u64::try_from(vector_count).unwrap_or(u64::MAX),
        "decompressing",
        args.no_progress,
    );
    for i in 0..vector_count {
        let cv = reader
            .next_vector()
            .map_err(|e| anyhow!("{e}"))
            .map_err(|e| e.context(CliErrorKind::Io))?
            .ok_or_else(|| {
                anyhow!("corpus truncated: expected {vector_count} records, got {i}")
                    .context(CliErrorKind::Verify)
            })?;
        let slice = out
            .get_mut(i * dim..(i + 1) * dim)
            .ok_or_else(|| anyhow!("internal: output slice indexing failed"))
            .map_err(|e| e.context(CliErrorKind::Other))?;
        codec
            .decompress_into(&cv, &config, &codebook, slice)
            .map_err(|e| anyhow!("{e}"))
            .map_err(|e| e.context(CliErrorKind::Other))?;
        bar.inc(1);
    }
    bar.finish();

    write_matrix(&args.output, args.format, vector_count, dim, &out)?;
    info!(path = %args.output.display(), "wrote decompressed matrix");

    // `threads` is accepted for CLI-contract stability but currently
    // unused — see module doc. Touch it to silence unused-warnings
    // once the `#[deny(warnings)]` lint is re-enabled at crate level.
    let _ = args.threads;
    Ok(())
}
