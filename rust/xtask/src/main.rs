//! `TinyQuant` workspace task runner.
//!
//! Run from the `rust/` directory: `cargo xtask <command>`
//!
//! Commands: `fmt` | `lint` | `test` | `help`
#![deny(warnings, clippy::all, clippy::pedantic)]

use std::process;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let task = args.get(1).map(String::as_str);
    match task {
        Some("fmt") => run("cargo", &["fmt", "--manifest-path", "Cargo.toml", "--all"]),
        Some("lint") => run(
            "cargo",
            &[
                "clippy",
                "--manifest-path",
                "Cargo.toml",
                "--workspace",
                "--all-targets",
                "--all-features",
                "--",
                "-D",
                "warnings",
            ],
        ),
        Some("test") => run(
            "cargo",
            &["test", "--manifest-path", "Cargo.toml", "--workspace"],
        ),
        Some("help") | None => print_help(),
        Some(t) => {
            eprintln!("unknown task: {t}");
            process::exit(1);
        }
    }
}

fn run(prog: &str, args: &[&str]) {
    let status = process::Command::new(prog)
        .args(args)
        .status()
        .unwrap_or_else(|e| {
            eprintln!("failed to spawn `{prog}`: {e}");
            process::exit(1);
        });
    if !status.success() {
        process::exit(status.code().unwrap_or(1));
    }
}

fn print_help() {
    println!("xtask — `TinyQuant` workspace task runner");
    println!();
    println!("Commands:");
    println!("  fmt    Format all Rust sources");
    println!("  lint   Run clippy with -D warnings");
    println!("  test   Run all workspace tests");
    println!("  help   Print this message (default)");
}
