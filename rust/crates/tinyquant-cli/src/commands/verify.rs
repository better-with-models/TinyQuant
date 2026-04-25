//! `tinyquant verify` — umbrella verifier that dispatches by magic bytes.
//!
//! Inspects the first few bytes of `path` and routes to the matching
//! structural verifier:
//!
//! - `TQCB` → codebook file ([`super::codebook_io::read_codebook`]).
//! - `TQCV` / `TQCX` → level-2 corpus file
//!   ([`tinyquant_io::codec_file::CodecFileReader`]). The reader
//!   parses the header, walks every record, and surfaces any
//!   truncation / checksum / magic issue.
//!
//! Exit codes (via [`CliErrorKind`]):
//!
//! - `0` — all checks passed.
//! - `3` — I/O error opening / reading the file.
//! - `4` — unknown magic or structural corruption.
//! - `70` — any other unexpected failure.

use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

use anyhow::{anyhow, Context, Result};
use tinyquant_io::codec_file::CodecFileReader;
use tracing::info;

use crate::commands::codebook_io::read_codebook;
use crate::commands::CliErrorKind;

/// Magic bytes for a codebook sidecar written by `codec train`.
const CODEBOOK_MAGIC: &[u8; 4] = b"TQCB";
/// Magic bytes for a finalized corpus file.
const CORPUS_FINAL_MAGIC: &[u8; 4] = b"TQCV";
/// Magic bytes for a corpus file that was still being written when the
/// producer process exited (the writer finalizes by flipping `TQCX`
/// back to `TQCV`).
const CORPUS_TENTATIVE_MAGIC: &[u8; 4] = b"TQCX";

/// Run `tinyquant verify <path>`.
///
/// # Errors
///
/// Returns [`anyhow::Error`] wrapping a [`CliErrorKind`] — see module
/// documentation for the exit-code mapping.
pub fn run(path: &Path) -> Result<()> {
    let magic = read_magic(path)?;

    match &magic {
        m if m == CODEBOOK_MAGIC => verify_codebook(path),
        m if m == CORPUS_FINAL_MAGIC || m == CORPUS_TENTATIVE_MAGIC => verify_corpus(path),
        other => Err(anyhow!(
            "{}: unknown magic {:02x?} (expected TQCB, TQCV, or TQCX)",
            path.display(),
            other
        ))
        .context(CliErrorKind::Verify),
    }
}

/// Read the first four bytes of `path`. Returns a Verify-tagged error
/// if the file is shorter than four bytes, and an Io-tagged error for
/// any other I/O failure.
fn read_magic(path: &Path) -> Result<[u8; 4]> {
    let file = File::open(path)
        .with_context(|| format!("opening {}", path.display()))
        .map_err(|e| e.context(CliErrorKind::Io))?;
    let mut r = BufReader::new(file);
    let mut magic = [0u8; 4];
    r.read_exact(&mut magic)
        .with_context(|| format!("reading magic from {}", path.display()))
        .map_err(|e| e.context(CliErrorKind::Verify))?;
    Ok(magic)
}

fn verify_codebook(path: &Path) -> Result<()> {
    let codebook = read_codebook(path)?;
    info!(
        path = %path.display(),
        bit_width = codebook.bit_width(),
        num_entries = codebook.num_entries(),
        "codebook verified"
    );
    Ok(())
}

fn verify_corpus(path: &Path) -> Result<()> {
    let file = File::open(path)
        .with_context(|| format!("opening corpus {}", path.display()))
        .map_err(|e| e.context(CliErrorKind::Io))?;
    let mut reader = CodecFileReader::new(BufReader::new(file))
        .map_err(|e| anyhow!("{e}"))
        .map_err(|e| e.context(CliErrorKind::Verify))?;
    // Snapshot the header fields we need up-front — `CorpusFileHeader`
    // doesn't implement `Clone`, and borrowing `reader.header()` would
    // conflict with the `&mut reader.next_vector()` below.
    let (expected_count, dimension, bit_width) = {
        let header = reader.header();
        (header.vector_count, header.dimension, header.bit_width)
    };
    let mut seen: u64 = 0;
    while let Some(_cv) = reader
        .next_vector()
        .map_err(|e| anyhow!("{e}"))
        .map_err(|e| e.context(CliErrorKind::Verify))?
    {
        seen = seen.saturating_add(1);
    }
    if seen != expected_count {
        return Err(anyhow!(
            "{}: record count mismatch — header says {expected_count}, read {seen}",
            path.display()
        ))
        .context(CliErrorKind::Verify);
    }
    info!(
        path = %path.display(),
        dimension,
        bit_width,
        vector_count = expected_count,
        "corpus verified"
    );
    Ok(())
}
