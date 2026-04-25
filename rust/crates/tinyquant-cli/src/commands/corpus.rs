//! Dispatch module for `tinyquant corpus ...`.

use anyhow::Result;

use crate::CorpusCmd;

use super::{codec_decompress, corpus_ingest, corpus_search};

/// Dispatch a parsed `corpus` subcommand.
///
/// # Errors
///
/// Propagates errors from the underlying `run` function.
pub fn dispatch(cmd: CorpusCmd, no_progress: bool) -> Result<()> {
    match cmd {
        CorpusCmd::Ingest {
            input,
            rows,
            cols,
            config_json,
            codebook,
            corpus_path,
            policy,
            threads,
            format,
        } => corpus_ingest::run(corpus_ingest::Args {
            input,
            rows,
            cols,
            config_json,
            codebook,
            corpus_path,
            policy,
            threads,
            format,
            no_progress,
        }),
        CorpusCmd::Decompress {
            corpus_path,
            output,
            codebook,
            config_json,
            format,
        } => {
            // `corpus decompress` is a thin wrapper over the codec
            // decompress path — identical file format, identical
            // record layout. The only difference is the flag names.
            let args = codec_decompress::Args {
                input: corpus_path,
                config_json: config_json.ok_or_else(|| {
                    anyhow::anyhow!("--config-json is required for corpus decompress")
                })?,
                codebook: codebook.ok_or_else(|| {
                    anyhow::anyhow!("--codebook is required for corpus decompress")
                })?,
                output,
                threads: 1,
                format,
                no_progress,
            };
            codec_decompress::run(args)
        }
        CorpusCmd::Search {
            corpus,
            query,
            codebook,
            config_json,
            top_k,
            format,
        } => corpus_search::run(corpus_search::Args {
            corpus,
            query,
            codebook,
            config_json,
            top_k,
            format,
            no_progress,
        }),
    }
}
