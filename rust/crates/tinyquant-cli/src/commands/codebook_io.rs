//! Read / write helpers for codebook + codec-config sidecar files.
//!
//! The codebook file format is a deliberately minimal, CLI-internal
//! contract (the on-wire codec layer uses the Level-1 / Level-2
//! protocols in `tinyquant-io`; this helper is *only* used by the CLI
//! to shuttle a trained codebook between `codec train` and
//! `codec compress` / `codec decompress`).
//!
//! **Binary layout** (little-endian):
//!
//! ```text
//! [0..4]   magic         b"TQCB"
//! [4]      format_version 0x01
//! [5]      bit_width     {2, 4, 8}
//! [6..8]   reserved      [0, 0]
//! [8..12]  num_entries   u32  (must equal 1 << bit_width)
//! [12..]   entries[..]   num_entries × f32 LE
//! ```
//!
//! **Config sidecar JSON**: a single object mirroring
//! [`CodecConfig`], consumed by the downstream subcommands to avoid
//! requiring the operator to pass the same 4 flags again.
//!
//! ```json
//! { "bit_width": 4, "seed": 42, "dimension": 64, "residual_enabled": true }
//! ```

use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::Path;

use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};
use tinyquant_core::codec::{Codebook, CodecConfig};

use crate::commands::CliErrorKind;

const CODEBOOK_MAGIC: &[u8; 4] = b"TQCB";
const CODEBOOK_VERSION: u8 = 0x01;

/// Persisted form of [`CodecConfig`].
///
/// `config_hash` is informational and not consumed on re-load —
/// `CodecConfig::new` recomputes it from the primary fields.
#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigSidecar {
    /// Bit width.
    pub bit_width: u8,
    /// RNG seed.
    pub seed: u64,
    /// Vector dimension.
    pub dimension: u32,
    /// Residual-enabled flag.
    pub residual_enabled: bool,
    /// Cached canonical hash (informational; recomputed on load).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config_hash: Option<String>,
}

impl From<&CodecConfig> for ConfigSidecar {
    fn from(cfg: &CodecConfig) -> Self {
        Self {
            bit_width: cfg.bit_width(),
            seed: cfg.seed(),
            dimension: cfg.dimension(),
            residual_enabled: cfg.residual_enabled(),
            config_hash: Some(cfg.config_hash().to_string()),
        }
    }
}

/// Write a codebook to `path` in the TQCB format.
///
/// # Errors
///
/// Returns an [`anyhow::Error`] tagged with [`CliErrorKind::Io`] on any
/// file-creation or write failure.
pub fn write_codebook(path: &Path, codebook: &Codebook) -> Result<()> {
    let file = File::create(path)
        .with_context(|| format!("creating codebook {}", path.display()))
        .map_err(|e| e.context(CliErrorKind::Io))?;
    let mut w = BufWriter::new(file);

    w.write_all(CODEBOOK_MAGIC)
        .and_then(|()| w.write_all(&[CODEBOOK_VERSION, codebook.bit_width(), 0, 0]))
        .and_then(|()| w.write_all(&codebook.num_entries().to_le_bytes()))
        .with_context(|| format!("writing codebook header {}", path.display()))
        .map_err(|e| e.context(CliErrorKind::Io))?;

    for v in codebook.entries() {
        w.write_all(&v.to_le_bytes())
            .with_context(|| format!("writing codebook entry {}", path.display()))
            .map_err(|e| e.context(CliErrorKind::Io))?;
    }
    w.flush()
        .with_context(|| format!("flushing codebook {}", path.display()))
        .map_err(|e| e.context(CliErrorKind::Io))
}

/// Read a codebook from `path`.
///
/// # Errors
///
/// Returns a [`CliErrorKind::Io`] or [`CliErrorKind::Verify`] tagged
/// error on I/O problems or magic / header mismatches.
pub fn read_codebook(path: &Path) -> Result<Codebook> {
    let file = File::open(path)
        .with_context(|| format!("opening codebook {}", path.display()))
        .map_err(|e| e.context(CliErrorKind::Io))?;
    let mut r = BufReader::new(file);
    let mut header = [0u8; 12];
    r.read_exact(&mut header)
        .with_context(|| format!("reading codebook header {}", path.display()))
        .map_err(|e| e.context(CliErrorKind::Io))?;

    // Destructure the fixed 12-byte header via pattern match so clippy
    // doesn't flag the slice indexing under `-D clippy::indexing_slicing`.
    let [m0, m1, m2, m3, version, bit_width, _r0, _r1, n0, n1, n2, n3] = header;
    let magic = [m0, m1, m2, m3];
    if &magic != CODEBOOK_MAGIC {
        return Err(anyhow!(
            "{}: bad codebook magic: got {:02x?}, expected TQCB",
            path.display(),
            magic
        ))
        .context(CliErrorKind::Verify);
    }
    if version != CODEBOOK_VERSION {
        return Err(anyhow!(
            "{}: unsupported codebook version {version}",
            path.display()
        ))
        .context(CliErrorKind::Verify);
    }
    let num_entries = u32::from_le_bytes([n0, n1, n2, n3]);

    let expected = 1u32.checked_shl(u32::from(bit_width)).unwrap_or(u32::MAX);
    if num_entries != expected {
        bail!(
            "{}: num_entries {num_entries} does not match 2^bit_width ({expected})",
            path.display()
        );
    }

    let mut entries: Vec<f32> = Vec::with_capacity(num_entries as usize);
    for _ in 0..num_entries {
        let mut buf = [0u8; 4];
        r.read_exact(&mut buf)
            .with_context(|| format!("reading codebook entry from {}", path.display()))
            .map_err(|e| e.context(CliErrorKind::Io))?;
        entries.push(f32::from_le_bytes(buf));
    }

    Codebook::new(entries.into_boxed_slice(), bit_width)
        .map_err(|e| anyhow!("{e}"))
        .map_err(|e| e.context(CliErrorKind::Verify))
}

/// Write a JSON sidecar describing a [`CodecConfig`].
///
/// # Errors
///
/// Returns [`CliErrorKind::Io`] on any file / write failure.
pub fn write_config_json(path: &Path, config: &CodecConfig) -> Result<()> {
    let sidecar = ConfigSidecar::from(config);
    let json = serde_json::to_string_pretty(&sidecar)
        .context("serializing config sidecar")
        .map_err(|e| e.context(CliErrorKind::Other))?;
    std::fs::write(path, json)
        .with_context(|| format!("writing config sidecar {}", path.display()))
        .map_err(|e| e.context(CliErrorKind::Io))
}

/// Read a JSON sidecar and reconstruct the underlying [`CodecConfig`].
///
/// # Errors
///
/// Returns [`CliErrorKind::Io`] on read failures and
/// [`CliErrorKind::InvalidArgs`] on parse or validation failures.
pub fn read_config_json(path: &Path) -> Result<CodecConfig> {
    let bytes = std::fs::read(path)
        .with_context(|| format!("reading config sidecar {}", path.display()))
        .map_err(|e| e.context(CliErrorKind::Io))?;
    let sidecar: ConfigSidecar = serde_json::from_slice(&bytes)
        .with_context(|| format!("parsing config sidecar {}", path.display()))
        .map_err(|e| e.context(CliErrorKind::InvalidArgs))?;
    CodecConfig::new(
        sidecar.bit_width,
        sidecar.seed,
        sidecar.dimension,
        sidecar.residual_enabled,
    )
    .map_err(|e| anyhow!("{e}"))
    .map_err(|e| e.context(CliErrorKind::InvalidArgs))
}
