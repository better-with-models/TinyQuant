//! Format-aware matrix I/O for the `tinyquant` CLI.
//!
//! Mirrors `docs/plans/rust/phase-22-pyo3-cabi-release.md` §CLI I/O
//! format specification. Every parser returns `(rows, cols, Vec<f32>)`
//! in row-major layout; every writer accepts the same tuple.
//!
//! Parsers validate:
//!
//! - f32 raw: `file.len() == rows * cols * 4`; `--rows`/`--cols`
//!   supplied.
//! - `.npy`: v1.0 format, dtype `float32`, shape 2-D, C-order.
//! - CSV: every cell parses with `f32::from_str`, row count consistent.
//! - JSONL: every line parses as `{"id": "...", "vector": [...]}`.
//!
//! Writers are symmetric. JSONL output emits a synthetic `id =
//! "{row_index}"` when the source format has no identifier.

use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Read, Write};
use std::path::Path;

use anyhow::{anyhow, bail, Context, Result};
use npyz::WriterBuilder;
use serde::{Deserialize, Serialize};

use crate::commands::CliErrorKind;
use crate::VectorFormat;

/// One-vector JSONL record. `id` and `metadata` round-trip verbatim.
#[derive(Debug, Serialize, Deserialize)]
pub struct JsonlRecord {
    /// Caller-supplied identifier. Synthesized as `"{row_index}"` when
    /// writing output that came from a format without ids.
    pub id: String,
    /// The vector itself.
    pub vector: Vec<f32>,
    /// Arbitrary passthrough metadata — preserved bit-for-bit across
    /// `jsonl → jsonl` round trips.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// Parse a matrix file into a row-major `Vec<f32>`.
///
/// # Arguments
///
/// - `path` — file to read.
/// - `format` — which parser to use.
/// - `rows_hint` / `cols_hint` — required for `--format f32`; ignored
///   by the other parsers (they get the shape from the file itself).
///
/// # Errors
///
/// Returns an [`anyhow::Error`] with a [`CliErrorKind::Io`] tag when
/// the file cannot be read, or [`CliErrorKind::InvalidArgs`] when the
/// supplied shape does not match the file contents.
pub fn read_matrix(
    path: &Path,
    format: VectorFormat,
    rows_hint: Option<usize>,
    cols_hint: Option<usize>,
) -> Result<(usize, usize, Vec<f32>)> {
    match format {
        VectorFormat::F32 => read_f32(path, rows_hint, cols_hint),
        VectorFormat::Npy => read_npy(path),
        VectorFormat::Csv => read_csv(path),
        VectorFormat::Jsonl => {
            let records = read_jsonl_records(path)?;
            let cols = records
                .first()
                .map(|r| r.vector.len())
                .ok_or_else(|| anyhow!("{}: empty jsonl file", path.display()))
                .map_err(|e| e.context(CliErrorKind::Io))?;
            let rows = records.len();
            let mut flat = Vec::with_capacity(rows * cols);
            for (i, rec) in records.into_iter().enumerate() {
                if rec.vector.len() != cols {
                    bail!(
                        "jsonl row {}: vector length {} differs from first row length {}",
                        i,
                        rec.vector.len(),
                        cols
                    );
                }
                flat.extend(rec.vector);
            }
            Ok((rows, cols, flat))
        }
    }
}

/// Write a row-major `Vec<f32>` out in the requested format.
pub fn write_matrix(
    path: &Path,
    format: VectorFormat,
    rows: usize,
    cols: usize,
    data: &[f32],
) -> Result<()> {
    if data.len() != rows.saturating_mul(cols) {
        bail!(
            "write_matrix: expected {} floats, got {}",
            rows.saturating_mul(cols),
            data.len()
        );
    }
    match format {
        VectorFormat::F32 => write_f32(path, data),
        VectorFormat::Npy => write_npy(path, rows, cols, data),
        VectorFormat::Csv => write_csv(path, cols, data),
        VectorFormat::Jsonl => write_jsonl(path, cols, data),
    }
}

fn read_f32(
    path: &Path,
    rows: Option<usize>,
    cols: Option<usize>,
) -> Result<(usize, usize, Vec<f32>)> {
    let rows = rows
        .ok_or_else(|| anyhow!("--rows is required with --format f32"))
        .map_err(|e| e.context(CliErrorKind::InvalidArgs))?;
    let cols = cols
        .ok_or_else(|| anyhow!("--cols is required with --format f32"))
        .map_err(|e| e.context(CliErrorKind::InvalidArgs))?;

    let bytes = std::fs::read(path)
        .with_context(|| format!("reading f32 matrix {}", path.display()))
        .map_err(|e| e.context(CliErrorKind::Io))?;
    let expected = rows
        .checked_mul(cols)
        .and_then(|n| n.checked_mul(4))
        .ok_or_else(|| anyhow!("rows * cols * 4 overflows usize"))
        .map_err(|e| e.context(CliErrorKind::InvalidArgs))?;
    if bytes.len() != expected {
        bail!(
            "{}: expected {} bytes for {}x{} f32 matrix, got {}",
            path.display(),
            expected,
            rows,
            cols,
            bytes.len()
        );
    }

    let mut out = vec![0.0f32; rows * cols];
    for (dst, chunk) in out.iter_mut().zip(bytes.chunks_exact(4)) {
        let mut buf = [0u8; 4];
        buf.copy_from_slice(chunk);
        *dst = f32::from_le_bytes(buf);
    }
    Ok((rows, cols, out))
}

fn write_f32(path: &Path, data: &[f32]) -> Result<()> {
    let mut w = BufWriter::new(
        File::create(path)
            .with_context(|| format!("creating {}", path.display()))
            .map_err(|e| e.context(CliErrorKind::Io))?,
    );
    for v in data {
        w.write_all(&v.to_le_bytes())
            .with_context(|| format!("writing {}", path.display()))
            .map_err(|e| e.context(CliErrorKind::Io))?;
    }
    w.flush()
        .with_context(|| format!("flushing {}", path.display()))
        .map_err(|e| e.context(CliErrorKind::Io))
}

fn read_npy(path: &Path) -> Result<(usize, usize, Vec<f32>)> {
    let mut bytes = Vec::new();
    File::open(path)
        .with_context(|| format!("opening {}", path.display()))
        .map_err(|e| e.context(CliErrorKind::Io))?
        .read_to_end(&mut bytes)
        .with_context(|| format!("reading {}", path.display()))
        .map_err(|e| e.context(CliErrorKind::Io))?;
    let reader = npyz::NpyFile::new(bytes.as_slice())
        .with_context(|| format!("parsing .npy header at {}", path.display()))
        .map_err(|e| e.context(CliErrorKind::InvalidArgs))?;

    let shape = reader.shape().to_vec();
    if shape.len() != 2 {
        return Err(anyhow!(
            "{}: expected 2-D float32 array, got shape {:?}",
            path.display(),
            shape
        ))
        .context(CliErrorKind::InvalidArgs);
    }
    if reader.order() != npyz::Order::C {
        return Err(anyhow!(
            "{}: Fortran-order arrays are not supported",
            path.display()
        ))
        .context(CliErrorKind::InvalidArgs);
    }
    let rows: usize = shape
        .first()
        .copied()
        .and_then(|n| usize::try_from(n).ok())
        .ok_or_else(|| anyhow!("npy rows dimension did not fit in usize"))
        .map_err(|e| e.context(CliErrorKind::InvalidArgs))?;
    let cols: usize = shape
        .get(1)
        .copied()
        .and_then(|n| usize::try_from(n).ok())
        .ok_or_else(|| anyhow!("npy cols dimension did not fit in usize"))
        .map_err(|e| e.context(CliErrorKind::InvalidArgs))?;

    let dtype = reader.dtype();
    let expected_ts: npyz::TypeStr = "<f4"
        .parse()
        .map_err(|e: npyz::ParseTypeStrError| anyhow::Error::new(e).context(CliErrorKind::Other))?;
    let expected_dtype = npyz::DType::Plain(expected_ts);
    if dtype != expected_dtype {
        return Err(anyhow!(
            "{}: expected dtype '<f4' (little-endian float32), got {:?}",
            path.display(),
            dtype
        ))
        .context(CliErrorKind::InvalidArgs);
    }

    let data: Vec<f32> = reader
        .into_vec::<f32>()
        .with_context(|| format!("decoding {}", path.display()))
        .map_err(|e| e.context(CliErrorKind::Io))?;
    Ok((rows, cols, data))
}

fn write_npy(path: &Path, rows: usize, cols: usize, data: &[f32]) -> Result<()> {
    let file = File::create(path)
        .with_context(|| format!("creating {}", path.display()))
        .map_err(|e| e.context(CliErrorKind::Io))?;
    let mut writer = BufWriter::new(file);
    let shape: [u64; 2] = [rows as u64, cols as u64];
    let mut npy = npyz::WriteOptions::new()
        .default_dtype()
        .shape(&shape)
        .writer(&mut writer)
        .begin_nd()
        .with_context(|| format!("begin npy header in {}", path.display()))
        .map_err(|e| e.context(CliErrorKind::Io))?;
    for v in data {
        npy.push(v)
            .with_context(|| format!("write npy row to {}", path.display()))
            .map_err(|e| e.context(CliErrorKind::Io))?;
    }
    npy.finish()
        .with_context(|| format!("finalize npy {}", path.display()))
        .map_err(|e| e.context(CliErrorKind::Io))?;
    writer
        .flush()
        .with_context(|| format!("flushing {}", path.display()))
        .map_err(|e| e.context(CliErrorKind::Io))
}

fn read_csv(path: &Path) -> Result<(usize, usize, Vec<f32>)> {
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(false)
        .flexible(false)
        .from_path(path)
        .with_context(|| format!("opening {}", path.display()))
        .map_err(|e| e.context(CliErrorKind::Io))?;
    let mut data: Vec<f32> = Vec::new();
    let mut cols: Option<usize> = None;
    let mut row_idx = 0usize;
    for record in rdr.records() {
        let record = record
            .with_context(|| format!("reading csv at row {row_idx}"))
            .map_err(|e| e.context(CliErrorKind::Io))?;
        match cols {
            None => cols = Some(record.len()),
            Some(n) if n != record.len() => {
                bail!(
                    "csv row {row_idx}: expected {n} cells, got {}",
                    record.len()
                );
            }
            Some(_) => {}
        }
        for (col_idx, cell) in record.iter().enumerate() {
            let v: f32 = cell.trim().parse().map_err(|_e| {
                anyhow::anyhow!("CSV parse error at row {row_idx}, column {col_idx}: '{cell}'")
                    .context(CliErrorKind::InvalidArgs)
            })?;
            data.push(v);
        }
        row_idx = row_idx.saturating_add(1);
    }
    let cols = cols.unwrap_or(0);
    Ok((row_idx, cols, data))
}

fn write_csv(path: &Path, cols: usize, data: &[f32]) -> Result<()> {
    let file = File::create(path)
        .with_context(|| format!("creating {}", path.display()))
        .map_err(|e| e.context(CliErrorKind::Io))?;
    let mut w = csv::WriterBuilder::new()
        .has_headers(false)
        .from_writer(BufWriter::new(file));
    for row in data.chunks(cols) {
        let cells: Vec<String> = row.iter().map(|v| format!("{v:.9}")).collect();
        w.write_record(&cells)
            .with_context(|| format!("writing {}", path.display()))
            .map_err(|e| e.context(CliErrorKind::Io))?;
    }
    w.flush()
        .with_context(|| format!("flushing {}", path.display()))
        .map_err(|e| e.context(CliErrorKind::Io))
}

/// Read JSONL records, preserving ids and metadata.
pub fn read_jsonl_records(path: &Path) -> Result<Vec<JsonlRecord>> {
    let file = File::open(path)
        .with_context(|| format!("opening {}", path.display()))
        .map_err(|e| e.context(CliErrorKind::Io))?;
    let reader = BufReader::new(file);
    let mut out = Vec::new();
    for (i, line) in reader.lines().enumerate() {
        let line = line
            .with_context(|| format!("reading {} line {i}", path.display()))
            .map_err(|e| e.context(CliErrorKind::Io))?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let rec: JsonlRecord = serde_json::from_str(trimmed)
            .with_context(|| format!("parsing jsonl line {i} in {}", path.display()))
            .map_err(|e| e.context(CliErrorKind::InvalidArgs))?;
        out.push(rec);
    }
    Ok(out)
}

fn write_jsonl(path: &Path, cols: usize, data: &[f32]) -> Result<()> {
    let file = File::create(path)
        .with_context(|| format!("creating {}", path.display()))
        .map_err(|e| e.context(CliErrorKind::Io))?;
    let mut w = BufWriter::new(file);
    for (i, row) in data.chunks(cols).enumerate() {
        let rec = JsonlRecord {
            id: i.to_string(),
            vector: row.to_vec(),
            metadata: None,
        };
        let json = serde_json::to_string(&rec)
            .context("serializing jsonl record")
            .map_err(|e| e.context(CliErrorKind::Other))?;
        writeln!(w, "{json}")
            .with_context(|| format!("writing {}", path.display()))
            .map_err(|e| e.context(CliErrorKind::Io))?;
    }
    w.flush()
        .with_context(|| format!("flushing {}", path.display()))
        .map_err(|e| e.context(CliErrorKind::Io))
}

/// Read a single FP32 vector (used by `corpus search --query`).
///
/// Expects either a raw little-endian `f32` file (whose length is a
/// multiple of 4 and whose vector is the file's full contents) or the
/// first record of a `.npy` file.
pub fn read_vector(path: &Path) -> Result<Vec<f32>> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if ext == "npy" {
        let (_r, cols, data) = read_npy(path)?;
        if data.len() != cols {
            bail!(
                "{}: expected a single-vector npy (1 row), got {} rows",
                path.display(),
                data.len() / cols.max(1)
            );
        }
        Ok(data)
    } else {
        let bytes = std::fs::read(path)
            .with_context(|| format!("reading {}", path.display()))
            .map_err(|e| e.context(CliErrorKind::Io))?;
        if bytes.len() % 4 != 0 {
            bail!(
                "{}: file length {} is not a multiple of 4 bytes",
                path.display(),
                bytes.len()
            );
        }
        let mut out = vec![0f32; bytes.len() / 4];
        for (dst, chunk) in out.iter_mut().zip(bytes.chunks_exact(4)) {
            let mut buf = [0u8; 4];
            buf.copy_from_slice(chunk);
            *dst = f32::from_le_bytes(buf);
        }
        Ok(out)
    }
}
