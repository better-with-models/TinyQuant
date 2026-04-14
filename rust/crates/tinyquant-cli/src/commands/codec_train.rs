//! `tinyquant codec train` — train a codebook from raw FP32 input.
//!
//! Pipeline:
//!
//! 1. Read the training matrix via [`crate::io::read_matrix`]
//!    (format-aware; `f32` / `npy` / `csv`).
//! 2. Build a [`tinyquant_core::codec::CodecConfig`] from the CLI flags.
//! 3. Call [`tinyquant_core::codec::Codebook::train`].
//! 4. Write the codebook in a self-describing binary format (see
//!    [`super::codebook_io`]) and optionally a JSON sidecar describing
//!    the [`CodecConfig`].

use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use tinyquant_core::codec::{Codebook, CodecConfig};
use tracing::info;

use crate::commands::codebook_io::{write_codebook, write_config_json};
use crate::commands::CliErrorKind;
use crate::io::read_matrix;
use crate::VectorFormat;

/// Arguments for `codec train`.
#[derive(Debug)]
pub struct Args {
    /// Training matrix path.
    pub input: PathBuf,
    /// Number of training vectors (required with `--format f32`).
    pub rows: usize,
    /// Dimension per vector.
    pub cols: usize,
    /// Quantization bit width (2, 4, 8).
    pub bit_width: u8,
    /// RNG seed.
    pub seed: u64,
    /// Residual flag.
    pub residual: bool,
    /// Input matrix format.
    pub format: VectorFormat,
    /// Codebook output path.
    pub output: PathBuf,
    /// Optional JSON sidecar describing the `CodecConfig`.
    pub config_out: Option<PathBuf>,
}

/// Run `tinyquant codec train`.
///
/// # Errors
///
/// Returns [`anyhow::Error`] wrapping a [`CliErrorKind`] for argument,
/// I/O, or codec failures.
pub fn run(args: Args) -> Result<()> {
    let (rows, cols, flat) =
        read_matrix(&args.input, args.format, Some(args.rows), Some(args.cols))?;

    if rows != args.rows || cols != args.cols {
        return Err(anyhow!(
            "training matrix shape mismatch: declared {}x{}, got {rows}x{cols}",
            args.rows,
            args.cols
        ))
        .context(CliErrorKind::InvalidArgs);
    }

    let cols_u32 = u32::try_from(cols)
        .map_err(|_| anyhow!("--cols {cols} does not fit in u32"))
        .map_err(|e| e.context(CliErrorKind::InvalidArgs))?;

    let config = CodecConfig::new(args.bit_width, args.seed, cols_u32, args.residual)
        .map_err(|e| anyhow!("{e}"))
        .map_err(|e| e.context(CliErrorKind::InvalidArgs))?;

    info!(
        rows,
        cols,
        bit_width = args.bit_width,
        seed = args.seed,
        residual = args.residual,
        "training codebook"
    );

    let codebook = Codebook::train(&flat, &config)
        .map_err(|e| anyhow!("{e}"))
        .map_err(|e| e.context(CliErrorKind::Other))?;

    write_codebook(&args.output, &codebook)?;
    info!(path = %args.output.display(), "wrote codebook");

    if let Some(path) = args.config_out.as_ref() {
        write_config_json(path, &config)?;
        info!(path = %path.display(), "wrote config sidecar");
    }

    Ok(())
}
