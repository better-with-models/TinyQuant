//! `tinyquant` command-line tool.
//!
//! Subcommand tree mirrors `docs/plans/rust/phase-22-pyo3-cabi-release.md`
//! §CLI subcommand reference (Phase 22.C). The clap derive layout here
//! is the single source of truth — per-subcommand logic lives in
//! [`commands`] modules, and format-aware I/O lives in [`io`].
//!
//! ## Exit codes
//!
//! `0` — success.
//! `2` — invalid arguments (clap default for parse failures).
//! `3` — I/O error (file not found, permission denied, read / write).
//! `4` — verify failed (bad magic, checksum, or header).
//! `70` — other (unexpected, mapped in [`main`] via [`anyhow`] + process exit).
//!
//! Error messages follow the shape `error: <category>: <detail>` and are
//! printed to stderr. Progress bars also go to stderr so
//! `tinyquant ... --format json > out.json` stays clean.

#![deny(
    missing_docs,
    unsafe_op_in_unsafe_fn,
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]
#![allow(
    clippy::module_name_repetitions,
    clippy::must_use_candidate,
    clippy::struct_excessive_bools,
    clippy::too_many_lines,
    clippy::too_many_arguments,
    // The nursery lints below disagree with the shape of this CLI's
    // error-reporting path (`anyhow::Error` must own its chain through
    // `report`, not a borrow; the `match handle.join()` is a load-bearing
    // readability choice, not a refactor candidate). Documenting the
    // allow-list explicitly rather than deleting the lint groups above
    // keeps the rest of the surface under the stricter defaults.
    clippy::option_if_let_else,
    clippy::single_match_else,
    clippy::needless_pass_by_value,
    clippy::doc_markdown,
)]

use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{CommandFactory, Parser, Subcommand, ValueEnum};

mod commands;
mod io;

/// Global allocator — jemalloc on non-MSVC hosts when the `jemalloc`
/// feature is enabled. This is a no-op on Windows / MSVC because
/// `jemallocator` is `cfg(not(target_env = "msvc"))` gated at the
/// dependency level.
#[cfg(all(feature = "jemalloc", not(target_env = "msvc")))]
#[global_allocator]
static GLOBAL: jemallocator::Jemalloc = jemallocator::Jemalloc;

/// Default CPU count for `--threads` arguments.
fn default_threads() -> usize {
    num_cpus::get().max(1)
}

/// Input / output encoding for vector matrices.
#[derive(Clone, Copy, Debug, ValueEnum)]
#[value(rename_all = "lowercase")]
pub enum VectorFormat {
    /// Raw little-endian `f32` bytes, row-major, no header.
    /// Requires `--rows` and `--cols`.
    F32,
    /// NumPy `.npy` v1.0 file. 2-D, dtype `float32`, C-order.
    Npy,
    /// CSV, one vector per row. Numeric cells only.
    Csv,
    /// JSON-lines: one `{"id","vector","metadata"}` object per line.
    Jsonl,
}

/// Output encoding for search results.
#[derive(Clone, Copy, Debug, ValueEnum)]
#[value(rename_all = "lowercase")]
pub enum SearchOutputFormat {
    /// Canonical JSON: `{"results":[{"id":"...","score":0.87}, ...]}`.
    Json,
    /// Human-readable ASCII table (respects `NO_COLOR`).
    Table,
    /// CSV with a header row.
    Csv,
}

/// Shells that `--generate-completion` understands.
#[derive(Clone, Copy, Debug, ValueEnum)]
#[value(rename_all = "lowercase")]
pub enum Shell {
    /// GNU Bash.
    Bash,
    /// Z Shell.
    Zsh,
    /// Friendly interactive shell.
    Fish,
    /// Microsoft PowerShell.
    Powershell,
}

/// `tinyquant` — CPU-only vector quantization codec.
#[derive(Parser, Debug)]
#[command(
    name = "tinyquant",
    bin_name = "tinyquant",
    version,
    about = "CPU-only vector quantization codec",
    long_about = None,
    disable_help_subcommand = true,
)]
pub struct Cli {
    /// `tracing` log filter.
    ///
    /// Accepts the standard `env_logger` / `tracing_subscriber::EnvFilter`
    /// syntax, e.g. `info`, `tinyquant_cli=debug`, or
    /// `warn,tinyquant_io=trace`.
    #[arg(long, global = true, env = "TINYQUANT_LOG", default_value = "info")]
    pub log: String,

    /// Emit shell completions to stdout and exit.
    #[arg(long = "generate-completion", value_enum, value_name = "SHELL")]
    pub generate_completion: Option<Shell>,

    /// Emit a `clap_mangen` man page to stdout and exit.
    #[arg(long = "generate-man")]
    pub generate_man: bool,

    /// Subcommand to dispatch.
    #[command(subcommand)]
    pub command: Option<Command>,
}

/// Top-level subcommand.
#[derive(Subcommand, Debug)]
pub enum Command {
    /// Print build metadata, feature set, and detected ISA.
    Info,

    /// Codec subcommands: `train`, `compress`, `decompress`.
    Codec {
        /// Codec-layer action.
        #[command(subcommand)]
        action: CodecCmd,
    },

    /// Corpus subcommands: `ingest`, `decompress`, `search`.
    Corpus {
        /// Corpus-layer action.
        #[command(subcommand)]
        action: CorpusCmd,
    },

    /// Verify a serialized `CompressedVector` or corpus file by magic bytes.
    Verify {
        /// Path to the file under test.
        path: PathBuf,
    },
}

/// `tinyquant codec ...`
#[derive(Subcommand, Debug)]
pub enum CodecCmd {
    /// Train a codebook from raw FP32 input.
    Train {
        /// Input FP32 matrix path.
        #[arg(long)]
        input: PathBuf,
        /// Number of training vectors.
        #[arg(long)]
        rows: usize,
        /// Dimension of each training vector. Must equal the codec dimension.
        #[arg(long)]
        cols: usize,
        /// Quantization bit width (one of 2, 4, 8).
        #[arg(long)]
        bit_width: u8,
        /// RNG seed for the codec config (propagates to `config_hash`).
        #[arg(long)]
        seed: u64,
        /// Enable residual (FP16) storage. Default `true` to match the
        /// Python reference.
        #[arg(long, default_value_t = true)]
        residual: bool,
        /// Input matrix format.
        #[arg(long, value_enum, default_value_t = VectorFormat::F32)]
        format: VectorFormat,
        /// Where to write the trained codebook (raw LE `f32` entries +
        /// 1-byte bit width; see `commands::codec_train`).
        #[arg(long)]
        output: PathBuf,
        /// Optional JSON sidecar describing the `CodecConfig` (consumed
        /// by `codec compress` / `codec decompress`). Extension of the
        /// §Step 12 skeleton: without it the downstream commands would
        /// need another way to reconstruct the config.
        #[arg(long = "config-out")]
        config_out: Option<PathBuf>,
    },
    /// Compress one or many vectors into a level-2 TQCV corpus file.
    Compress {
        /// Input FP32 matrix path.
        #[arg(long)]
        input: PathBuf,
        /// Number of vectors (required when `--format f32`).
        #[arg(long)]
        rows: Option<usize>,
        /// Dimension per vector (required when `--format f32`).
        #[arg(long)]
        cols: Option<usize>,
        /// JSON sidecar emitted by `codec train --config-out`.
        #[arg(long)]
        config_json: PathBuf,
        /// Codebook file emitted by `codec train`.
        #[arg(long)]
        codebook: PathBuf,
        /// Output TQCV corpus file path.
        #[arg(long)]
        output: PathBuf,
        /// Thread count for the batch compress pool.
        #[arg(long, default_value_t = default_threads())]
        threads: usize,
        /// Input matrix format.
        #[arg(long, value_enum, default_value_t = VectorFormat::F32)]
        format: VectorFormat,
    },
    /// Decompress a TQCV corpus file back to raw FP32.
    Decompress {
        /// Input TQCV corpus file path.
        #[arg(long)]
        input: PathBuf,
        /// JSON sidecar describing the `CodecConfig`.
        #[arg(long)]
        config_json: PathBuf,
        /// Codebook file.
        #[arg(long)]
        codebook: PathBuf,
        /// Output FP32 file path.
        #[arg(long)]
        output: PathBuf,
        /// Thread count for parallel decompress.
        #[arg(long, default_value_t = default_threads())]
        threads: usize,
        /// Output matrix format (f32 / npy / csv).
        #[arg(long, value_enum, default_value_t = VectorFormat::F32)]
        format: VectorFormat,
    },
}

/// `tinyquant corpus ...`
#[derive(Subcommand, Debug)]
pub enum CorpusCmd {
    /// Ingest an FP32 matrix into a new corpus file under a chosen policy.
    Ingest {
        /// Input FP32 matrix path.
        #[arg(long)]
        input: PathBuf,
        /// Number of vectors (required when `--format f32`).
        #[arg(long)]
        rows: Option<usize>,
        /// Dimension per vector (required when `--format f32`).
        #[arg(long)]
        cols: Option<usize>,
        /// JSON sidecar describing the `CodecConfig` (only required for
        /// `--policy compress`).
        #[arg(long)]
        config_json: Option<PathBuf>,
        /// Codebook file (only required for `--policy compress`).
        #[arg(long)]
        codebook: Option<PathBuf>,
        /// Target corpus file path.
        #[arg(long)]
        corpus_path: PathBuf,
        /// Compression policy.
        #[arg(long, value_enum)]
        policy: IngestPolicy,
        /// Thread count for batch ingest.
        #[arg(long, default_value_t = default_threads())]
        threads: usize,
        /// Input matrix format.
        #[arg(long, value_enum, default_value_t = VectorFormat::F32)]
        format: VectorFormat,
    },
    /// Decompress every vector in a corpus file into a raw FP32 matrix.
    Decompress {
        /// Input corpus file path.
        #[arg(long)]
        corpus_path: PathBuf,
        /// Output FP32 matrix path.
        #[arg(long)]
        output: PathBuf,
        /// Codebook file (required when the corpus was written with
        /// the `compress` policy).
        #[arg(long)]
        codebook: Option<PathBuf>,
        /// JSON sidecar describing the `CodecConfig` (required with
        /// the `compress` policy).
        #[arg(long)]
        config_json: Option<PathBuf>,
        /// Output matrix format.
        #[arg(long, value_enum, default_value_t = VectorFormat::F32)]
        format: VectorFormat,
    },
    /// Brute-force search a query vector against a corpus file.
    Search {
        /// Input corpus file path.
        #[arg(long)]
        corpus: PathBuf,
        /// Path to a single-vector FP32 file (dimension must match the
        /// corpus).
        #[arg(long)]
        query: PathBuf,
        /// Codebook file (required for `compress`-policy corpora).
        #[arg(long)]
        codebook: Option<PathBuf>,
        /// JSON sidecar describing the `CodecConfig`.
        #[arg(long)]
        config_json: Option<PathBuf>,
        /// Top-K results to return.
        #[arg(long, default_value_t = 10)]
        top_k: usize,
        /// Output format for search results.
        #[arg(long, value_enum, default_value_t = SearchOutputFormat::Json)]
        format: SearchOutputFormat,
    },
}

/// Corpus compression policy for `corpus ingest`.
#[derive(Clone, Copy, Debug, ValueEnum)]
#[value(rename_all = "lowercase")]
pub enum IngestPolicy {
    /// Full codec pipeline (requires `--codebook` + `--config-json`).
    Compress,
    /// Store raw FP32 (1x compression ratio).
    Passthrough,
    /// Store FP16 (2x compression ratio).
    Fp16,
}

/// Stack size for the worker thread that hosts `real_main`.
///
/// Windows ships with a 1 MiB default main-thread stack, which is
/// insufficient for the faer QR decomposition path used by
/// `RotationMatrix::build` in debug builds. Spawning the CLI body on
/// an explicitly sized thread (8 MiB) keeps `cargo test` green on
/// Windows without forcing every downstream test run into `--release`.
/// On Linux / macOS 8 MiB is already the default, so this is
/// effectively a no-op there.
const WORKER_STACK_SIZE: usize = 8 * 1024 * 1024;

fn main() -> ExitCode {
    // Parse args on the main thread so `--help` / bad args still
    // terminate with clap's default ExitCode 2 without the cost of a
    // thread spawn.
    let cli = Cli::parse();

    // --generate-completion / --generate-man short-circuit before any
    // subcommand dispatch, per §Step 17.
    if let Some(shell) = cli.generate_completion {
        return match commands::generate_completion(shell) {
            Ok(()) => ExitCode::SUCCESS,
            Err(err) => report(err),
        };
    }
    if cli.generate_man {
        return match commands::generate_man() {
            Ok(()) => ExitCode::SUCCESS,
            Err(err) => report(err),
        };
    }

    // Hand the heavyweight dispatch off to a worker thread with an
    // 8 MiB stack. See `WORKER_STACK_SIZE` docs for the rationale.
    let builder = std::thread::Builder::new()
        .name("tinyquant-main".into())
        .stack_size(WORKER_STACK_SIZE);
    match builder.spawn(move || real_main(cli)) {
        Ok(handle) => match handle.join() {
            Ok(code) => code,
            Err(_) => {
                let stderr = std::io::stderr();
                let mut h = stderr.lock();
                let _ = writeln!(h, "error: tinyquant-main thread panicked");
                ExitCode::from(EXIT_CODE_OTHER)
            }
        },
        Err(e) => {
            let stderr = std::io::stderr();
            let mut h = stderr.lock();
            let _ = writeln!(h, "error: spawning worker thread: {e}");
            ExitCode::from(EXIT_CODE_OTHER)
        }
    }
}

/// Fallback exit code when something goes wrong outside the normal
/// `anyhow::Error -> CliErrorKind` flow (thread spawn / join errors).
const EXIT_CODE_OTHER: u8 = 70;

fn real_main(cli: Cli) -> ExitCode {
    if let Err(err) = init_tracing(&cli.log) {
        return report(err);
    }

    let Some(command) = cli.command else {
        let _ = Cli::command().print_help();
        return ExitCode::from(2);
    };

    match dispatch(command) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => report(err),
    }
}

/// Top-level dispatch. Each arm is a thin wrapper over the
/// corresponding `commands::*::run` entry point.
fn dispatch(command: Command) -> anyhow::Result<()> {
    match command {
        Command::Info => commands::info::run(),
        Command::Codec { action } => commands::codec::dispatch(action),
        Command::Corpus { action } => commands::corpus::dispatch(action),
        Command::Verify { path } => commands::verify::run(&path),
    }
}

/// Emit a formatted error to stderr and return a non-zero exit code.
///
/// The exit code is derived from the error chain — the caller wraps
/// any `std::io::Error` via [`anyhow::Context`] and we unwrap it here
/// to distinguish "I/O" (exit 3) from "verify" (exit 4) from "other"
/// (exit 70).
fn report(err: anyhow::Error) -> ExitCode {
    let kind = err
        .chain()
        .find_map(|e| e.downcast_ref::<commands::CliErrorKind>())
        .copied()
        .unwrap_or(commands::CliErrorKind::Other);

    let stderr = std::io::stderr();
    let mut handle = stderr.lock();
    let _ = writeln!(handle, "error: {err:#}");

    ExitCode::from(kind.exit_code())
}

fn init_tracing(filter: &str) -> anyhow::Result<()> {
    use tracing_subscriber::{fmt, EnvFilter};

    let env_filter = EnvFilter::try_new(filter)
        .map_err(|e| anyhow::anyhow!("invalid --log filter '{filter}': {e}"))?;

    // `try_init` is a no-op if the subscriber is already set, which
    // happens inside integration tests that share a process.
    let _ = fmt()
        .with_env_filter(env_filter)
        .with_writer(std::io::stderr)
        .with_target(false)
        .try_init();
    Ok(())
}
