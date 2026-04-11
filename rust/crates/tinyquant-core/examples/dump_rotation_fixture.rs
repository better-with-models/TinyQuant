//! Example binary that writes a canonical rotation matrix to a
//! little-endian `f64` binary file.
//!
//! Usage:
//!
//! ```text
//! cargo run -p tinyquant-core --example dump_rotation_fixture --features std -- \
//!     <seed> <dimension> <out-path>
//! ```
//!
//! Produces `<out-path>`, a raw little-endian `f64[dim * dim]` stream
//! laid out row-major. The file format is consumed by the
//! `rotation_fixture_parity` integration test and by downstream tooling
//! that wants to diff Rust fixtures against arbitrary future backends.
//!
//! The example is gated behind the `std` feature so that
//! `cargo build -p tinyquant-core --no-default-features` (the embedded
//! no_std check) does not attempt to build it.

use std::env;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::process::ExitCode;
use std::thread;

use tinyquant_core::codec::RotationMatrix;

/// faer's debug-mode QR frames are deep; give the worker thread 32 MiB
/// so users can run this via `cargo run` without a release build.
const WORKER_STACK_BYTES: usize = 32 * 1024 * 1024;

fn main() -> ExitCode {
    let handle = match thread::Builder::new()
        .name(String::from("dump_rotation_fixture"))
        .stack_size(WORKER_STACK_BYTES)
        .spawn(run)
    {
        Ok(h) => h,
        Err(err) => {
            eprintln!("failed to spawn worker: {err}");
            return ExitCode::from(2);
        }
    };
    match handle.join() {
        Ok(code) => code,
        Err(_) => {
            eprintln!("worker panicked");
            ExitCode::from(2)
        }
    }
}

fn run() -> ExitCode {
    let mut args = env::args().skip(1);
    let seed: u64 = match args.next().and_then(|s| s.parse().ok()) {
        Some(v) => v,
        None => return usage("missing or invalid <seed>"),
    };
    let dim: u32 = match args.next().and_then(|s| s.parse().ok()) {
        Some(v) if v > 0 => v,
        _ => return usage("missing or invalid <dimension>"),
    };
    let out_path: PathBuf = match args.next() {
        Some(s) => PathBuf::from(s),
        None => return usage("missing <out-path>"),
    };

    if let Some(parent) = out_path.parent() {
        if !parent.as_os_str().is_empty() {
            if let Err(err) = fs::create_dir_all(parent) {
                eprintln!("failed to create {}: {err}", parent.display());
                return ExitCode::from(2);
            }
        }
    }

    let rot = RotationMatrix::build(seed, dim);
    let matrix = rot.matrix();
    debug_assert_eq!(matrix.len(), (dim as usize) * (dim as usize));

    let file = match File::create(&out_path) {
        Ok(f) => f,
        Err(err) => {
            eprintln!("failed to create {}: {err}", out_path.display());
            return ExitCode::from(2);
        }
    };
    let mut writer = BufWriter::new(file);
    for value in matrix {
        if let Err(err) = writer.write_all(&value.to_le_bytes()) {
            eprintln!("write error at {}: {err}", out_path.display());
            return ExitCode::from(2);
        }
    }
    if let Err(err) = writer.flush() {
        eprintln!("flush error at {}: {err}", out_path.display());
        return ExitCode::from(2);
    }

    println!(
        "wrote seed={seed} dim={dim} ({} bytes) -> {}",
        matrix.len() * 8,
        out_path.display()
    );
    ExitCode::SUCCESS
}

fn usage(msg: &str) -> ExitCode {
    eprintln!("error: {msg}");
    eprintln!("usage: dump_rotation_fixture <seed> <dimension> <out-path>");
    ExitCode::from(1)
}
