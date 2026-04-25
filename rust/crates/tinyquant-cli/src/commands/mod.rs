//! Subcommand modules + shared error taxonomy.
//!
//! Each `commands::<name>::run` function is a small, tracing-
//! instrumented wrapper over the `tinyquant-core` / `tinyquant-io` /
//! `tinyquant-bruteforce` APIs. Structured errors are attached via
//! [`anyhow::Context`] carrying a [`CliErrorKind`] discriminant, which
//! `main::report` unwraps to choose the process exit code.

use std::io::{self, Write};

use anyhow::{Context, Result};
use clap::CommandFactory;

use crate::{Cli, Shell};

pub mod codebook_io;
pub mod codec;
pub mod codec_compress;
pub mod codec_decompress;
pub mod codec_train;
pub mod corpus;
pub mod corpus_ingest;
pub mod corpus_search;
pub mod info;
pub mod verify;

/// Error category for `main::report` — attached to `anyhow::Error`
/// chains via [`anyhow::Context::context`]. The lowest-numbered code
/// in the chain wins (i.e. the context nearest the root cause).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CliErrorKind {
    /// Invalid arguments — wrong shape, parse error, missing sidecar.
    /// Mapped to exit code `2`.
    InvalidArgs,
    /// I/O error — file not found, permission denied, read / write.
    /// Mapped to exit code `3`.
    Io,
    /// Verify failure — magic byte mismatch, checksum mismatch.
    /// Mapped to exit code `4`.
    Verify,
    /// Fallback. Mapped to exit code `70`.
    Other,
}

impl CliErrorKind {
    /// The POSIX-flavoured exit code for this kind.
    #[must_use]
    pub const fn exit_code(self) -> u8 {
        match self {
            Self::InvalidArgs => 2,
            Self::Io => 3,
            Self::Verify => 4,
            Self::Other => 70,
        }
    }
}

impl std::fmt::Display for CliErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidArgs => f.write_str("invalid arguments"),
            Self::Io => f.write_str("io"),
            Self::Verify => f.write_str("verify failed"),
            Self::Other => f.write_str("error"),
        }
    }
}

impl std::error::Error for CliErrorKind {}

/// Emit shell completions to stdout (§Step 17).
pub fn generate_completion(shell: Shell) -> Result<()> {
    use clap_complete::{generate, shells};
    let mut cmd = Cli::command();
    let name = cmd.get_name().to_string();
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    match shell {
        Shell::Bash => generate(shells::Bash, &mut cmd, &name, &mut handle),
        Shell::Zsh => generate(shells::Zsh, &mut cmd, &name, &mut handle),
        Shell::Fish => generate(shells::Fish, &mut cmd, &name, &mut handle),
        Shell::Powershell => generate(shells::PowerShell, &mut cmd, &name, &mut handle),
    }
    handle
        .flush()
        .context("flushing stdout")
        .map_err(|e| e.context(CliErrorKind::Io))?;
    Ok(())
}

/// Emit a `clap_mangen` man page to stdout (§Step 17).
pub fn generate_man() -> Result<()> {
    let cmd = Cli::command();
    let man = clap_mangen::Man::new(cmd);
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    man.render(&mut handle)
        .context("rendering man page")
        .map_err(|e| e.context(CliErrorKind::Other))?;
    handle
        .flush()
        .context("flushing stdout")
        .map_err(|e| e.context(CliErrorKind::Io))?;
    Ok(())
}
