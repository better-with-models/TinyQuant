//! Standalone Rust helper for the cross-surface TinyQuant benchmark.

use std::{
    env,
    error::Error,
    fs,
    path::{Path, PathBuf},
    sync::Arc,
    time::Instant,
};

use ndarray::Array2;
use serde::{Deserialize, Serialize};
use tinyquant_bruteforce::BruteForceBackend;
use tinyquant_core::{
    backend::SearchBackend,
    codec::{Codec, CodecConfig, Codebook, CompressedVector, PreparedCodec},
};
use tinyquant_gpu_wgpu::{ComputeBackend, TinyQuantGpuError, WgpuBackend};

type DynError = Box<dyn Error + Send + Sync + 'static>;

#[derive(Debug, Deserialize)]
struct BenchmarkSpec {
    codec_config: CodecConfigSpec,
    warmups: usize,
    reps: usize,
    top_k: usize,
    query_count: usize,
    surfaces: Vec<String>,
    suites: Vec<String>,
    corpora: Vec<CorpusSpec>,
}

#[derive(Debug, Clone, Deserialize)]
struct CodecConfigSpec {
    bit_width: u8,
    seed: u64,
    dimension: u32,
    residual_enabled: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
struct CorpusSpec {
    id: String,
    label: String,
    rows: usize,
    dim: usize,
    source: String,
    codec_path: String,
    search_path: String,
    query_indices: Vec<usize>,
}

#[derive(Debug, Clone)]
struct LoadedCorpus {
    spec: CorpusSpec,
    codec_data: Vec<f32>,
    search_data: Vec<f32>,
}

#[derive(Debug, Serialize)]
struct HelperPayload {
    environment: HelperEnvironment,
    surface_statuses: Vec<SurfaceStatusRow>,
    rows: Vec<ResultRow>,
}

#[derive(Debug, Default, Serialize)]
struct HelperEnvironment {
    wgpu_adapter_name: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct SurfaceStatusRow {
    surface_id: String,
    surface_label: String,
    status: String,
    reason: Option<String>,
    details: String,
}

#[derive(Debug, Clone, Serialize)]
struct ResultRow {
    suite: String,
    surface_id: String,
    surface_label: String,
    corpus_id: String,
    rows: usize,
    dim: usize,
    phase: String,
    metric_unit: String,
    median: Option<f64>,
    min: Option<f64>,
    max: Option<f64>,
    samples: Vec<f64>,
    warmups: usize,
    reps: usize,
    status: String,
    skip_reason: Option<String>,
    notes: String,
}

struct Args {
    spec: PathBuf,
    out: PathBuf,
}

struct WgpuHarness {
    runtime: tokio::runtime::Runtime,
    adapter_name: Option<String>,
}

enum SummaryField {
    Median,
    Min,
    Max,
}

fn main() -> Result<(), DynError> {
    let args = Args::parse(env::args().skip(1))?;
    let spec = read_spec(&args.spec)?;
    let corpora = spec
        .corpora
        .iter()
        .map(LoadedCorpus::load)
        .collect::<Result<Vec<_>, _>>()?;

    let mut rows = Vec::new();
    let mut surface_statuses = Vec::new();
    let mut helper_environment = HelperEnvironment::default();

    if spec.surfaces.iter().any(|surface| surface == "rust_cpu") {
        surface_statuses.push(SurfaceStatusRow::ok("rust_cpu"));
        run_rust_cpu(&spec, &corpora, &mut rows)?;
    }

    if spec.surfaces.iter().any(|surface| surface == "rust_wgpu") {
        match WgpuHarness::new() {
            Ok(harness) => {
                helper_environment.wgpu_adapter_name = harness.adapter_name.clone();
                surface_statuses.push(SurfaceStatusRow {
                    surface_id: "rust_wgpu".to_string(),
                    surface_label: surface_label("rust_wgpu").to_string(),
                    status: "ok".to_string(),
                    reason: None,
                    details: helper_environment
                        .wgpu_adapter_name
                        .clone()
                        .unwrap_or_default(),
                });
                run_rust_wgpu(&spec, &corpora, &mut rows, harness)?;
            }
            Err(TinyQuantGpuError::NoAdapter) => {
                let reason = "no wgpu adapter available";
                surface_statuses.push(SurfaceStatusRow::skipped("rust_wgpu", reason));
                rows.extend(skipped_rows_for_surface(&spec, "rust_wgpu", &corpora, reason));
            }
            Err(err) => return Err(Box::new(err)),
        }
    }

    let payload = HelperPayload {
        environment: helper_environment,
        surface_statuses,
        rows,
    };
    fs::write(&args.out, serde_json::to_vec_pretty(&payload)?)?;
    Ok(())
}

impl Args {
    fn parse<I>(mut args: I) -> Result<Self, DynError>
    where
        I: Iterator<Item = String>,
    {
        let mut spec = None;
        let mut out = None;
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--spec" => spec = args.next().map(PathBuf::from),
                "--out" => out = args.next().map(PathBuf::from),
                _ => return Err(format!("unknown argument: {arg}").into()),
            }
        }
        Ok(Self {
            spec: spec.ok_or("missing --spec")?,
            out: out.ok_or("missing --out")?,
        })
    }
}

impl LoadedCorpus {
    fn load(spec: &CorpusSpec) -> Result<Self, DynError> {
        let codec_data = load_array2(Path::new(&spec.codec_path), spec.rows, spec.dim)?;
        let search_data = load_array2(Path::new(&spec.search_path), spec.rows, spec.dim)?;
        for &index in &spec.query_indices {
            if index >= spec.rows {
                return Err(format!(
                    "query index {index} out of bounds for corpus {} with {} rows",
                    spec.id, spec.rows
                )
                .into());
            }
        }
        Ok(Self {
            spec: spec.clone(),
            codec_data,
            search_data,
        })
    }

    fn query_views(&self) -> Vec<&[f32]> {
        self.spec
            .query_indices
            .iter()
            .map(|&index| self.search_row(index))
            .collect()
    }

    fn search_row(&self, row: usize) -> &[f32] {
        let start = row * self.spec.dim;
        let end = start + self.spec.dim;
        &self.search_data[start..end]
    }
}

impl WgpuHarness {
    fn new() -> Result<Self, TinyQuantGpuError> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("tokio runtime");
        let backend = runtime.block_on(WgpuBackend::new())?;
        let adapter_name = backend.adapter_name().map(ToOwned::to_owned);
        drop(backend);
        Ok(Self {
            runtime,
            adapter_name,
        })
    }

    fn new_backend(&self) -> Result<WgpuBackend, TinyQuantGpuError> {
        self.runtime.block_on(WgpuBackend::new())
    }
}

impl SurfaceStatusRow {
    fn ok(surface_id: &str) -> Self {
        Self {
            surface_id: surface_id.to_string(),
            surface_label: surface_label(surface_id).to_string(),
            status: "ok".to_string(),
            reason: None,
            details: String::new(),
        }
    }

    fn skipped(surface_id: &str, reason: &str) -> Self {
        Self {
            surface_id: surface_id.to_string(),
            surface_label: surface_label(surface_id).to_string(),
            status: "skipped".to_string(),
            reason: Some(reason.to_string()),
            details: String::new(),
        }
    }
}

fn read_spec(path: &Path) -> Result<BenchmarkSpec, DynError> {
    Ok(serde_json::from_str(&fs::read_to_string(path)?)?)
}

fn load_array2(path: &Path, expected_rows: usize, expected_dim: usize) -> Result<Vec<f32>, DynError> {
    let array: Array2<f32> = ndarray_npy::read_npy(path)?;
    let shape = array.shape();
    if shape != [expected_rows, expected_dim] {
        return Err(format!(
            "unexpected corpus shape for {}: expected ({expected_rows}, {expected_dim}), got ({}, {})",
            path.display(),
            shape[0],
            shape[1]
        )
        .into());
    }
    Ok(array.iter().copied().collect())
}

fn run_rust_cpu(
    spec: &BenchmarkSpec,
    corpora: &[LoadedCorpus],
    rows: &mut Vec<ResultRow>,
) -> Result<(), DynError> {
    for corpus in corpora {
        for suite in &spec.suites {
            match suite.as_str() {
                "codec" => match run_rust_cpu_codec_case(spec, corpus) {
                    Ok(case_rows) => rows.extend(case_rows),
                    Err(err) => rows.extend(failed_rows(spec, "rust_cpu", &corpus.spec, "codec", &err.to_string())),
                },
                "search" => match run_rust_cpu_search_case(spec, corpus) {
                    Ok(case_rows) => rows.extend(case_rows),
                    Err(err) => rows.extend(failed_rows(spec, "rust_cpu", &corpus.spec, "search", &err.to_string())),
                },
                other => return Err(format!("unsupported suite: {other}").into()),
            }
        }
    }
    Ok(())
}

fn run_rust_cpu_codec_case(
    spec: &BenchmarkSpec,
    corpus: &LoadedCorpus,
) -> Result<Vec<ResultRow>, DynError> {
    let config = build_codec_config(&spec.codec_config)?;
    let codec = Codec::new();

    // Setup: measure codebook training only (no rotation matrix involved).
    run_warmups(spec.warmups, || -> Result<(), DynError> {
        let _ = Codebook::train(&corpus.codec_data, &config)?;
        Ok(())
    })?;
    let setup_samples = measure_reps(spec.reps, || -> Result<(), DynError> {
        let _ = Codebook::train(&corpus.codec_data, &config)?;
        Ok(())
    })?;

    // Build PreparedCodec once: this pays the O(dim²) rotation QR exactly once
    // and caches it for all compress/decompress calls below (hot path).
    // compress_batch / decompress_batch_into use the cold path that rebuilds the
    // rotation per vector, making them impractically slow at dim=1536.  We
    // implement the same batch logic here using the prepared hot-path APIs.
    let codebook = Codebook::train(&corpus.codec_data, &config)?;
    let prepared = PreparedCodec::new(config.clone(), codebook)?;

    // Hot-path batch compress: rotation computed once via PreparedCodec.
    let compress_hot = |codec_ref: &Codec,
                        data: &[f32],
                        rows: usize,
                        cols: usize,
                        prep: &PreparedCodec|
     -> Result<Vec<CompressedVector>, DynError> {
        (0..rows)
            .map(|i| {
                #[allow(clippy::indexing_slicing)]
                let row = &data[i * cols..(i + 1) * cols];
                Ok(codec_ref.compress_prepared(row, prep)?)
            })
            .collect()
    };

    run_warmups(spec.warmups, || -> Result<(), DynError> {
        let _ = compress_hot(
            &codec,
            &corpus.codec_data,
            corpus.spec.rows,
            corpus.spec.dim,
            &prepared,
        )?;
        Ok(())
    })?;
    let compress_samples = measure_reps(spec.reps, || -> Result<(), DynError> {
        let _ = compress_hot(
            &codec,
            &corpus.codec_data,
            corpus.spec.rows,
            corpus.spec.dim,
            &prepared,
        )?;
        Ok(())
    })?;

    // Hot-path batch decompress: rotation computed once via PreparedCodec.
    let compressed = compress_hot(
        &codec,
        &corpus.codec_data,
        corpus.spec.rows,
        corpus.spec.dim,
        &prepared,
    )?;
    let decompress_hot = |codec_ref: &Codec,
                          cvs: &[CompressedVector],
                          cols: usize,
                          prep: &PreparedCodec|
     -> Result<(), DynError> {
        let mut out = vec![0.0_f32; cols];
        for cv in cvs {
            codec_ref.decompress_prepared_into(cv, prep, &mut out)?;
        }
        Ok(())
    };
    run_warmups(spec.warmups, || {
        decompress_hot(&codec, &compressed, corpus.spec.dim, &prepared)
    })?;
    let decompress_samples = measure_reps(spec.reps, || {
        decompress_hot(&codec, &compressed, corpus.spec.dim, &prepared)
    })?;

    Ok(vec![
        successful_row(
            "codec",
            "rust_cpu",
            &corpus.spec,
            "codec_setup",
            setup_samples,
            spec.warmups,
            spec.reps,
            "codebook training only",
        ),
        successful_row(
            "codec",
            "rust_cpu",
            &corpus.spec,
            "codec_compress_batch",
            compress_samples,
            spec.warmups,
            spec.reps,
            "hot-path batch compress via PreparedCodec (rotation cached once)",
        ),
        successful_row(
            "codec",
            "rust_cpu",
            &corpus.spec,
            "codec_decompress_batch",
            decompress_samples,
            spec.warmups,
            spec.reps,
            "hot-path batch decompress via PreparedCodec (rotation cached once)",
        ),
    ])
}

fn run_rust_cpu_search_case(
    spec: &BenchmarkSpec,
    corpus: &LoadedCorpus,
) -> Result<Vec<ResultRow>, DynError> {
    let entries = corpus_entries(corpus);

    run_warmups(spec.warmups, || -> Result<(), DynError> {
        let mut backend = BruteForceBackend::new();
        backend.ingest(&entries)?;
        Ok(())
    })?;
    let setup_samples = measure_reps(spec.reps, || -> Result<(), DynError> {
        let mut backend = BruteForceBackend::new();
        backend.ingest(&entries)?;
        Ok(())
    })?;

    let mut backend = BruteForceBackend::new();
    backend.ingest(&entries)?;

    run_warmups(spec.warmups, || search_batch_cpu(&backend, corpus, spec.top_k).map(|_| ()))?;
    let (batch_samples, p50_samples, p95_samples) =
        measure_search_reps(spec.reps, || search_batch_cpu(&backend, corpus, spec.top_k))?;

    let query_notes = format!("query_count={}, top_k={}", spec.query_count, spec.top_k);
    Ok(vec![
        successful_row(
            "search",
            "rust_cpu",
            &corpus.spec,
            "search_setup",
            setup_samples,
            spec.warmups,
            spec.reps,
            "backend construction plus ingest",
        ),
        successful_row(
            "search",
            "rust_cpu",
            &corpus.spec,
            "search_query_batch",
            batch_samples,
            spec.warmups,
            spec.reps,
            &query_notes,
        ),
        successful_row(
            "search",
            "rust_cpu",
            &corpus.spec,
            "search_query_p50",
            p50_samples,
            spec.warmups,
            spec.reps,
            &query_notes,
        ),
        successful_row(
            "search",
            "rust_cpu",
            &corpus.spec,
            "search_query_p95",
            p95_samples,
            spec.warmups,
            spec.reps,
            &query_notes,
        ),
    ])
}

fn run_rust_wgpu(
    spec: &BenchmarkSpec,
    corpora: &[LoadedCorpus],
    rows: &mut Vec<ResultRow>,
    harness: WgpuHarness,
) -> Result<(), DynError> {
    for corpus in corpora {
        for suite in &spec.suites {
            match suite.as_str() {
                "codec" => match run_rust_wgpu_codec_case(spec, corpus, &harness) {
                    Ok(case_rows) => rows.extend(case_rows),
                    Err(err) => rows.extend(failed_rows(spec, "rust_wgpu", &corpus.spec, "codec", &err.to_string())),
                },
                "search" => match run_rust_wgpu_search_case(spec, corpus, &harness) {
                    Ok(case_rows) => rows.extend(case_rows),
                    Err(err) => rows.extend(failed_rows(spec, "rust_wgpu", &corpus.spec, "search", &err.to_string())),
                },
                other => return Err(format!("unsupported suite: {other}").into()),
            }
        }
    }
    Ok(())
}

fn run_rust_wgpu_codec_case(
    spec: &BenchmarkSpec,
    corpus: &LoadedCorpus,
    harness: &WgpuHarness,
) -> Result<Vec<ResultRow>, DynError> {
    let config = build_codec_config(&spec.codec_config)?;

    run_warmups(spec.warmups, || -> Result<(), DynError> {
        let codebook = Codebook::train(&corpus.codec_data, &config)?;
        let mut prepared = PreparedCodec::new(config.clone(), codebook)?;
        let mut backend = harness.new_backend()?;
        backend.prepare_for_device(&mut prepared)?;
        Ok(())
    })?;
    let setup_samples = measure_reps(spec.reps, || -> Result<(), DynError> {
        let codebook = Codebook::train(&corpus.codec_data, &config)?;
        let mut prepared = PreparedCodec::new(config.clone(), codebook)?;
        let mut backend = harness.new_backend()?;
        backend.prepare_for_device(&mut prepared)?;
        Ok(())
    })?;

    let codebook = Codebook::train(&corpus.codec_data, &config)?;
    let mut prepared = PreparedCodec::new(config.clone(), codebook)?;
    let mut backend = harness.new_backend()?;
    backend.prepare_for_device(&mut prepared)?;

    run_warmups(spec.warmups, || -> Result<(), DynError> {
        let _ = backend.compress_batch(
            &corpus.codec_data,
            corpus.spec.rows,
            corpus.spec.dim,
            &prepared,
        )?;
        Ok(())
    })?;
    let compress_samples = measure_reps(spec.reps, || -> Result<(), DynError> {
        let _ = backend.compress_batch(
            &corpus.codec_data,
            corpus.spec.rows,
            corpus.spec.dim,
            &prepared,
        )?;
        Ok(())
    })?;

    let compressed = backend.compress_batch(
        &corpus.codec_data,
        corpus.spec.rows,
        corpus.spec.dim,
        &prepared,
    )?;
    run_warmups(spec.warmups, || -> Result<(), DynError> {
        let mut out = vec![0.0_f32; corpus.spec.rows * corpus.spec.dim];
        backend.decompress_batch_into(&compressed, &prepared, &mut out)?;
        Ok(())
    })?;
    let decompress_samples = measure_reps(spec.reps, || -> Result<(), DynError> {
        let mut out = vec![0.0_f32; corpus.spec.rows * corpus.spec.dim];
        backend.decompress_batch_into(&compressed, &prepared, &mut out)?;
        Ok(())
    })?;

    Ok(vec![
        successful_row(
            "codec",
            "rust_wgpu",
            &corpus.spec,
            "codec_setup",
            setup_samples,
            spec.warmups,
            spec.reps,
            "codebook training plus PreparedCodec::new plus prepare_for_device",
        ),
        successful_row(
            "codec",
            "rust_wgpu",
            &corpus.spec,
            "codec_compress_batch",
            compress_samples,
            spec.warmups,
            spec.reps,
            "GPU compress_batch on prepared device state",
        ),
        successful_row(
            "codec",
            "rust_wgpu",
            &corpus.spec,
            "codec_decompress_batch",
            decompress_samples,
            spec.warmups,
            spec.reps,
            "GPU decompress_batch_into from pre-compressed payload",
        ),
    ])
}

fn run_rust_wgpu_search_case(
    spec: &BenchmarkSpec,
    corpus: &LoadedCorpus,
    harness: &WgpuHarness,
) -> Result<Vec<ResultRow>, DynError> {
    run_warmups(spec.warmups, || -> Result<(), DynError> {
        let mut backend = harness.new_backend()?;
        backend.prepare_corpus_for_device(&corpus.search_data, corpus.spec.rows, corpus.spec.dim)?;
        Ok(())
    })?;
    let setup_samples = measure_reps(spec.reps, || -> Result<(), DynError> {
        let mut backend = harness.new_backend()?;
        backend.prepare_corpus_for_device(&corpus.search_data, corpus.spec.rows, corpus.spec.dim)?;
        Ok(())
    })?;

    let mut backend = harness.new_backend()?;
    backend.prepare_corpus_for_device(&corpus.search_data, corpus.spec.rows, corpus.spec.dim)?;

    run_warmups(spec.warmups, || search_batch_wgpu(&mut backend, corpus, spec.top_k).map(|_| ()))?;
    let (batch_samples, p50_samples, p95_samples) =
        measure_search_reps(spec.reps, || search_batch_wgpu(&mut backend, corpus, spec.top_k))?;

    let query_notes = format!("query_count={}, top_k={}", spec.query_count, spec.top_k);
    Ok(vec![
        successful_row(
            "search",
            "rust_wgpu",
            &corpus.spec,
            "search_setup",
            setup_samples,
            spec.warmups,
            spec.reps,
            "backend construction plus prepare_corpus_for_device",
        ),
        successful_row(
            "search",
            "rust_wgpu",
            &corpus.spec,
            "search_query_batch",
            batch_samples,
            spec.warmups,
            spec.reps,
            &query_notes,
        ),
        successful_row(
            "search",
            "rust_wgpu",
            &corpus.spec,
            "search_query_p50",
            p50_samples,
            spec.warmups,
            spec.reps,
            &query_notes,
        ),
        successful_row(
            "search",
            "rust_wgpu",
            &corpus.spec,
            "search_query_p95",
            p95_samples,
            spec.warmups,
            spec.reps,
            &query_notes,
        ),
    ])
}

fn corpus_entries(corpus: &LoadedCorpus) -> Vec<(Arc<str>, Vec<f32>)> {
    (0..corpus.spec.rows)
        .map(|row| {
            let id: Arc<str> = Arc::from(row.to_string());
            let start = row * corpus.spec.dim;
            let end = start + corpus.spec.dim;
            (id, corpus.search_data[start..end].to_vec())
        })
        .collect()
}

fn search_batch_cpu(
    backend: &BruteForceBackend,
    corpus: &LoadedCorpus,
    top_k: usize,
) -> Result<Vec<f64>, DynError> {
    let mut per_query_ms = Vec::with_capacity(corpus.spec.query_indices.len());
    for query in corpus.query_views() {
        let start = Instant::now();
        let _ = backend.search(query, top_k)?;
        per_query_ms.push(elapsed_ms(start));
    }
    Ok(per_query_ms)
}

fn search_batch_wgpu(
    backend: &mut WgpuBackend,
    corpus: &LoadedCorpus,
    top_k: usize,
) -> Result<Vec<f64>, DynError> {
    let mut per_query_ms = Vec::with_capacity(corpus.spec.query_indices.len());
    for query in corpus.query_views() {
        let start = Instant::now();
        let _ = backend.cosine_topk(query, top_k)?;
        per_query_ms.push(elapsed_ms(start));
    }
    Ok(per_query_ms)
}

fn build_codec_config(spec: &CodecConfigSpec) -> Result<CodecConfig, DynError> {
    Ok(CodecConfig::new(
        spec.bit_width,
        spec.seed,
        spec.dimension,
        spec.residual_enabled,
    )?)
}

fn run_warmups<F>(warmups: usize, mut f: F) -> Result<(), DynError>
where
    F: FnMut() -> Result<(), DynError>,
{
    for _ in 0..warmups {
        f()?;
    }
    Ok(())
}

fn measure_reps<F>(reps: usize, mut f: F) -> Result<Vec<f64>, DynError>
where
    F: FnMut() -> Result<(), DynError>,
{
    let mut samples = Vec::with_capacity(reps);
    for _ in 0..reps {
        let start = Instant::now();
        f()?;
        samples.push(elapsed_ms(start));
    }
    Ok(samples)
}

fn measure_search_reps<F>(reps: usize, mut f: F) -> Result<(Vec<f64>, Vec<f64>, Vec<f64>), DynError>
where
    F: FnMut() -> Result<Vec<f64>, DynError>,
{
    let mut batch_samples = Vec::with_capacity(reps);
    let mut p50_samples = Vec::with_capacity(reps);
    let mut p95_samples = Vec::with_capacity(reps);
    for _ in 0..reps {
        let batch_start = Instant::now();
        let per_query_ms = f()?;
        batch_samples.push(elapsed_ms(batch_start));
        p50_samples.push(percentile_linear(&per_query_ms, 50.0)?);
        p95_samples.push(percentile_linear(&per_query_ms, 95.0)?);
    }
    Ok((batch_samples, p50_samples, p95_samples))
}

fn elapsed_ms(start: Instant) -> f64 {
    start.elapsed().as_secs_f64() * 1000.0
}

fn percentile_linear(values: &[f64], percentile: f64) -> Result<f64, DynError> {
    if values.is_empty() {
        return Err("cannot compute percentile of empty sample set".into());
    }
    if values.len() == 1 {
        return Ok(values[0]);
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(|left, right| left.total_cmp(right));
    let rank = (percentile / 100.0) * ((sorted.len() - 1) as f64);
    let lower = rank.floor() as usize;
    let upper = rank.ceil() as usize;
    if lower == upper {
        return Ok(sorted[lower]);
    }
    let weight = rank - lower as f64;
    Ok(sorted[lower] + weight * (sorted[upper] - sorted[lower]))
}

fn successful_row(
    suite: &str,
    surface_id: &str,
    corpus: &CorpusSpec,
    phase: &str,
    samples: Vec<f64>,
    warmups: usize,
    reps: usize,
    notes: &str,
) -> ResultRow {
    ResultRow {
        suite: suite.to_string(),
        surface_id: surface_id.to_string(),
        surface_label: surface_label(surface_id).to_string(),
        corpus_id: corpus.id.clone(),
        rows: corpus.rows,
        dim: corpus.dim,
        phase: phase.to_string(),
        metric_unit: "ms".to_string(),
        median: summarize_samples(&samples, SummaryField::Median),
        min: summarize_samples(&samples, SummaryField::Min),
        max: summarize_samples(&samples, SummaryField::Max),
        samples,
        warmups,
        reps,
        status: "ok".to_string(),
        skip_reason: None,
        notes: notes.to_string(),
    }
}

fn summarize_samples(samples: &[f64], field: SummaryField) -> Option<f64> {
    if samples.is_empty() {
        return None;
    }
    match field {
        SummaryField::Median => percentile_linear(samples, 50.0).ok(),
        SummaryField::Min => samples.iter().copied().reduce(f64::min),
        SummaryField::Max => samples.iter().copied().reduce(f64::max),
    }
}

fn skipped_rows_for_surface(
    spec: &BenchmarkSpec,
    surface_id: &str,
    corpora: &[LoadedCorpus],
    reason: &str,
) -> Vec<ResultRow> {
    let mut rows = Vec::new();
    for corpus in corpora {
        for suite in &spec.suites {
            rows.extend(status_rows(
                suite,
                surface_id,
                &corpus.spec,
                suite_phases(suite),
                spec.warmups,
                spec.reps,
                "skipped",
                reason,
            ));
        }
    }
    rows
}

fn failed_rows(
    spec: &BenchmarkSpec,
    surface_id: &str,
    corpus: &CorpusSpec,
    suite: &str,
    reason: &str,
) -> Vec<ResultRow> {
    status_rows(
        suite,
        surface_id,
        corpus,
        suite_phases(suite),
        spec.warmups,
        spec.reps,
        "failed",
        reason,
    )
}

fn status_rows(
    suite: &str,
    surface_id: &str,
    corpus: &CorpusSpec,
    phases: &[&str],
    warmups: usize,
    reps: usize,
    status: &str,
    reason: &str,
) -> Vec<ResultRow> {
    phases
        .iter()
        .map(|phase| ResultRow {
            suite: suite.to_string(),
            surface_id: surface_id.to_string(),
            surface_label: surface_label(surface_id).to_string(),
            corpus_id: corpus.id.clone(),
            rows: corpus.rows,
            dim: corpus.dim,
            phase: (*phase).to_string(),
            metric_unit: "ms".to_string(),
            median: None,
            min: None,
            max: None,
            samples: Vec::new(),
            warmups,
            reps,
            status: status.to_string(),
            skip_reason: Some(reason.to_string()),
            notes: String::new(),
        })
        .collect()
}

fn suite_phases(suite: &str) -> &[&str] {
    match suite {
        "codec" => &[
            "codec_setup",
            "codec_compress_batch",
            "codec_decompress_batch",
        ],
        "search" => &[
            "search_setup",
            "search_query_batch",
            "search_query_p50",
            "search_query_p95",
        ],
        _ => &[],
    }
}

fn surface_label(surface_id: &str) -> &'static str {
    match surface_id {
        "rust_cpu" => "Rust CPU",
        "rust_wgpu" => "Rust wgpu",
        "py_reference" => "Python reference",
        "py_shim" => "Python shim",
        "py_direct" => "Python direct",
        _ => "Unknown",
    }
}
