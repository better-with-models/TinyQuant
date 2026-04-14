//! `indicatif`-backed progress bar helpers for long-running batch paths
//! (§Step 14 of `docs/plans/rust/phase-22-pyo3-cabi-release.md`).
//!
//! The CLI's feature flag matrix pins the behaviour:
//!
//! - `progress` feature **on** (default): `indicatif::ProgressBar`
//!   renders to stderr for `codec compress`, `codec decompress`,
//!   `corpus ingest`, and `corpus search`.
//! - `progress` feature **off**: the entire module compiles to a set
//!   of no-op wrappers — no `indicatif` dependency, no runtime cost.
//! - `--no-progress` flag (global): operator-level opt-out; forces a
//!   hidden bar even when the feature is on.
//! - `NO_COLOR=1` environment variable: disables bar colorization
//!   (handled by `indicatif` itself when it detects the variable).
//! - `TERM=dumb` environment variable: disables bars entirely, per
//!   the feature matrix table in the plan doc.
//!
//! ## Architecture
//!
//! The per-row rayon driver in
//! [`crate::commands::codec_compress`] is a bare `fn` pointer and
//! cannot close over per-call state. To let the driver tick a progress
//! bar we publish the currently-active bar through a
//! [`std::sync::OnceLock`]-wrapped [`std::sync::Mutex`] in this
//! module. `set_active_compress_bar` installs an optional bar before
//! `pool.install(...)`, `tick_active_compress_bar` nudges it from the
//! rayon workers, and `clear_active_compress_bar` tears it back down
//! when the call returns. The pattern is single-threaded at the
//! install / clear boundary (only one batch at a time runs in the
//! CLI) so the mutex contention is negligible.

#[cfg(feature = "progress")]
use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};
use std::sync::{Mutex, OnceLock};

/// Opaque handle returned by [`bar`].
///
/// Owns the underlying `indicatif::ProgressBar` when `progress` is
/// enabled, and degrades to an empty struct otherwise. The `tick`,
/// `inc`, and `finish` methods are no-ops on the disabled path so call
/// sites do not need feature gates.
pub struct Progress {
    #[cfg(feature = "progress")]
    inner: Option<ProgressBar>,
}

impl Progress {
    /// Advance by `n`. No-op when the bar is disabled.
    #[cfg(feature = "progress")]
    pub fn inc(&self, n: u64) {
        if let Some(bar) = &self.inner {
            bar.inc(n);
        }
    }

    /// Advance by `n`. No-op when the bar is disabled.
    #[cfg(not(feature = "progress"))]
    pub fn inc(&self, _n: u64) {}

    /// Finish the bar and clear the line. No-op when disabled.
    #[cfg(feature = "progress")]
    pub fn finish(self) {
        if let Some(bar) = self.inner {
            bar.finish_and_clear();
        }
    }

    /// Finish the bar and clear the line. No-op when disabled.
    #[cfg(not(feature = "progress"))]
    pub const fn finish(self) {}

    /// Borrow the inner `indicatif` bar, if the `progress` feature is
    /// on and the bar is enabled. Used by
    /// [`set_active_compress_bar`] to publish the bar to the rayon
    /// driver.
    #[cfg(feature = "progress")]
    pub const fn inner(&self) -> Option<&ProgressBar> {
        self.inner.as_ref()
    }
}

/// Build a determinate progress bar with length `len`, labelled with
/// `label`. Honours the `progress` feature, `--no-progress`,
/// `NO_COLOR=1`, and `TERM=dumb`.
#[cfg(feature = "progress")]
pub fn bar(len: u64, label: &str, no_progress: bool) -> Progress {
    if no_progress || is_dumb_term() {
        return Progress { inner: None };
    }
    let bar = ProgressBar::new(len)
        .with_style(
            ProgressStyle::with_template("  {msg:<14} [{bar:30}] {pos}/{len} ({eta})")
                .unwrap_or_else(|_| ProgressStyle::default_bar())
                .progress_chars("=> "),
        )
        .with_message(label.to_owned());
    bar.set_draw_target(ProgressDrawTarget::stderr());
    Progress { inner: Some(bar) }
}

/// Build a determinate progress bar with length `len`, labelled with
/// `label`. Compiles to a no-op when the `progress` feature is off.
#[cfg(not(feature = "progress"))]
#[allow(clippy::missing_const_for_fn)]
pub fn bar(_len: u64, _label: &str, _no_progress: bool) -> Progress {
    Progress {}
}

/// Returns `true` when `TERM=dumb` is set — a hint from screen-reader
/// terminals / CI scrollback buffers that ANSI progress updates are
/// unwanted.
#[cfg(feature = "progress")]
fn is_dumb_term() -> bool {
    std::env::var_os("TERM").is_some_and(|t| t == "dumb")
}

/// Globally-published progress bar for the rayon-driven
/// `codec compress` batch path. See module docs for the rationale.
#[cfg(feature = "progress")]
static COMPRESS_PROGRESS: OnceLock<Mutex<Option<ProgressBar>>> = OnceLock::new();

/// No-progress stub — kept so the `codec_compress` module can call
/// `clear_active_compress_bar` in its cleanup path without gating.
#[cfg(not(feature = "progress"))]
static COMPRESS_PROGRESS: OnceLock<Mutex<Option<()>>> = OnceLock::new();

/// Install `progress` as the active rayon-driver-side bar for the
/// duration of a `compress_batch_with` call. Passing `None` clears it
/// (equivalent to [`clear_active_compress_bar`]).
#[cfg(feature = "progress")]
pub fn set_active_compress_bar(progress: Option<&Progress>) {
    let slot = COMPRESS_PROGRESS.get_or_init(|| Mutex::new(None));
    if let Ok(mut guard) = slot.lock() {
        *guard = progress.and_then(Progress::inner).cloned();
    }
}

/// Install `progress` as the active rayon-driver-side bar for the
/// duration of a `compress_batch_with` call. No-op without `progress`.
#[cfg(not(feature = "progress"))]
pub fn set_active_compress_bar(_progress: Option<&Progress>) {}

/// Clear the active rayon-driver-side bar. Safe to call unconditionally
/// at the end of a batch.
pub fn clear_active_compress_bar() {
    #[cfg(feature = "progress")]
    {
        if let Some(slot) = COMPRESS_PROGRESS.get() {
            if let Ok(mut guard) = slot.lock() {
                *guard = None;
            }
        }
    }
}

/// Tick the active rayon-driver-side bar by one row. No-op when no
/// bar is installed or the `progress` feature is off.
pub fn tick_active_compress_bar() {
    #[cfg(feature = "progress")]
    {
        if let Some(slot) = COMPRESS_PROGRESS.get() {
            if let Ok(guard) = slot.lock() {
                if let Some(bar) = guard.as_ref() {
                    bar.inc(1);
                }
            }
        }
    }
}
