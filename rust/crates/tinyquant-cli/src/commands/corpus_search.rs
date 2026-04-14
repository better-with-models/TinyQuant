//! `tinyquant corpus search` — brute-force top-k search over a TQCV
//! corpus.
//!
//! Pipeline:
//!
//! 1. Load the config sidecar + codebook (required: corpora are
//!    codec-compressed).
//! 2. Stream every record through `Codec::decompress_into` into an
//!    [`tinyquant_bruteforce::BruteForceBackend`], using the record
//!    index stringified as the vector id (`"0"`, `"1"`, …).
//! 3. Load the query from `--query` via [`crate::io::read_vector`].
//! 4. Call `backend.search(&query, top_k)` and format the results
//!    according to `--format` (`json` / `table` / `csv`).

use std::fs::File;
use std::io::{self, BufReader, Write};
use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use tinyquant_bruteforce::BruteForceBackend;
use tinyquant_core::backend::SearchBackend;
use tinyquant_core::codec::Codec;
use tinyquant_core::types::VectorId;
use tinyquant_io::codec_file::CodecFileReader;
use tracing::info;

use crate::commands::codebook_io::{read_codebook, read_config_json};
use crate::commands::CliErrorKind;
use crate::io::read_vector;
use crate::progress;
use crate::SearchOutputFormat;

/// Arguments for `corpus search`.
#[derive(Debug)]
pub struct Args {
    /// Corpus path.
    pub corpus: PathBuf,
    /// Query vector path.
    pub query: PathBuf,
    /// Codebook (required — corpus is codec-compressed).
    pub codebook: Option<PathBuf>,
    /// Config sidecar (required).
    pub config_json: Option<PathBuf>,
    /// Top-K.
    pub top_k: usize,
    /// Output format for results.
    pub format: SearchOutputFormat,
    /// Suppress `indicatif` progress bar (propagated from the global
    /// `--no-progress` flag).
    pub no_progress: bool,
}

/// Run `tinyquant corpus search`.
///
/// # Errors
///
/// Returns [`anyhow::Error`] wrapping a [`CliErrorKind`] for argument,
/// I/O, or backend failures.
pub fn run(args: Args) -> Result<()> {
    let config_json = args.config_json.ok_or_else(|| {
        anyhow!("--config-json is required for corpus search").context(CliErrorKind::InvalidArgs)
    })?;
    let codebook_path = args.codebook.ok_or_else(|| {
        anyhow!("--codebook is required for corpus search").context(CliErrorKind::InvalidArgs)
    })?;

    let config = read_config_json(&config_json)?;
    let codebook = read_codebook(&codebook_path)?;
    if codebook.bit_width() != config.bit_width() {
        return Err(anyhow!(
            "codebook bit_width {} != config bit_width {}",
            codebook.bit_width(),
            config.bit_width()
        ))
        .context(CliErrorKind::Verify);
    }

    let file = File::open(&args.corpus)
        .with_context(|| format!("opening corpus {}", args.corpus.display()))
        .map_err(|e| e.context(CliErrorKind::Io))?;
    let mut reader = CodecFileReader::new(BufReader::new(file))
        .map_err(|e| anyhow!("{e}"))
        .map_err(|e| e.context(CliErrorKind::Verify))?;
    let dim = reader.header().dimension as usize;
    if reader.header().dimension != config.dimension() {
        return Err(anyhow!(
            "corpus dimension {} != config dimension {}",
            reader.header().dimension,
            config.dimension()
        ))
        .context(CliErrorKind::Verify);
    }

    let codec = Codec::new();
    let mut backend = BruteForceBackend::new();
    let mut batch: Vec<(VectorId, Vec<f32>)> = Vec::new();
    let mut idx: u64 = 0;
    // §Step 14: corpus search streams the whole corpus through
    // decompress before querying. For big corpora this is the
    // long-running leg, so we show a determinate bar keyed off the
    // header-declared vector count.
    let total_records = reader.header().vector_count;
    let bar = progress::bar(total_records, "loading corpus", args.no_progress);
    while let Some(cv) = reader
        .next_vector()
        .map_err(|e| anyhow!("{e}"))
        .map_err(|e| e.context(CliErrorKind::Io))?
    {
        let mut vec_out = vec![0.0f32; dim];
        codec
            .decompress_into(&cv, &config, &codebook, &mut vec_out)
            .map_err(|e| anyhow!("{e}"))
            .map_err(|e| e.context(CliErrorKind::Other))?;
        batch.push((VectorId::from(idx.to_string()), vec_out));
        idx = idx.saturating_add(1);
        bar.inc(1);
    }
    bar.finish();
    backend
        .ingest(&batch)
        .map_err(|e| anyhow!("{e}"))
        .map_err(|e| e.context(CliErrorKind::Other))?;
    info!(
        corpus = %args.corpus.display(),
        rows = backend.len(),
        top_k = args.top_k,
        "backend ready"
    );

    let query = read_vector(&args.query)?;
    if query.len() != dim {
        return Err(anyhow!(
            "query length {} != corpus dimension {dim}",
            query.len()
        ))
        .context(CliErrorKind::InvalidArgs);
    }

    let results = backend
        .search(&query, args.top_k)
        .map_err(|e| anyhow!("{e}"))
        .map_err(|e| e.context(CliErrorKind::Other))?;

    let stdout = io::stdout();
    let mut handle = stdout.lock();
    match args.format {
        SearchOutputFormat::Json => write_json(&mut handle, &results)?,
        SearchOutputFormat::Table => write_table(&mut handle, &results)?,
        SearchOutputFormat::Csv => write_csv(&mut handle, &results)?,
    }
    handle
        .flush()
        .context("flushing stdout")
        .map_err(|e| e.context(CliErrorKind::Io))?;
    Ok(())
}

fn write_json<W: Write>(
    w: &mut W,
    results: &[tinyquant_core::backend::SearchResult],
) -> Result<()> {
    let payload = serde_json::json!({
        "results": results
            .iter()
            .map(|r| {
                serde_json::json!({
                    "id": r.vector_id.to_string(),
                    "score": r.score,
                })
            })
            .collect::<Vec<_>>(),
    });
    serde_json::to_writer(&mut *w, &payload)
        .context("writing json results")
        .map_err(|e| e.context(CliErrorKind::Io))?;
    writeln!(w)
        .context("newline")
        .map_err(|e| e.context(CliErrorKind::Io))?;
    Ok(())
}

fn write_table<W: Write>(
    w: &mut W,
    results: &[tinyquant_core::backend::SearchResult],
) -> Result<()> {
    use comfy_table::{Cell, Table};

    let mut table = Table::new();
    table.set_header(vec!["rank", "id", "score"]);
    for (i, r) in results.iter().enumerate() {
        table.add_row(vec![
            Cell::new(i + 1),
            Cell::new(r.vector_id.to_string()),
            Cell::new(format!("{:.6}", r.score)),
        ]);
    }
    writeln!(w, "{table}")
        .context("writing table")
        .map_err(|e| e.context(CliErrorKind::Io))?;
    Ok(())
}

fn write_csv<W: Write>(w: &mut W, results: &[tinyquant_core::backend::SearchResult]) -> Result<()> {
    let mut writer = csv::WriterBuilder::new().has_headers(false).from_writer(w);
    writer
        .write_record(["rank", "id", "score"])
        .context("csv header")
        .map_err(|e| e.context(CliErrorKind::Io))?;
    for (i, r) in results.iter().enumerate() {
        writer
            .write_record([
                (i + 1).to_string(),
                r.vector_id.to_string(),
                format!("{:.9}", r.score),
            ])
            .context("csv row")
            .map_err(|e| e.context(CliErrorKind::Io))?;
    }
    writer
        .flush()
        .context("csv flush")
        .map_err(|e| e.context(CliErrorKind::Io))?;
    Ok(())
}
