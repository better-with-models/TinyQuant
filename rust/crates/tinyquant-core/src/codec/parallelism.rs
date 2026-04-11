//! Parallelism dispatch for batch codec methods (Phase 15).
//!
//! Phase 15 implements only [`Parallelism::Serial`]; `Custom` is the
//! escape hatch for Phase 21's rayon integration.

/// Strategy for dispatching row-parallel work inside batch codec methods.
#[derive(Clone, Copy, Default)]
pub enum Parallelism {
    /// Execute rows sequentially on the current thread.
    #[default]
    Serial,
    /// Defer to a caller-supplied driver (e.g. rayon).
    Custom(fn(count: usize, body: &(dyn Fn(usize) + Sync + Send))),
}

impl Parallelism {
    /// Drive `body` once per row, honoring the selected strategy.
    pub fn for_each_row<F>(self, count: usize, body: F)
    where
        F: Fn(usize) + Sync + Send,
    {
        match self {
            Self::Serial => (0..count).for_each(body),
            Self::Custom(driver) => driver(count, &body),
        }
    }
}
