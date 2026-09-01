//! Cooperative cancellation for long geometry operations.
//!
//! # Why cooperative
//!
//! `GeomError::Cancelled` existed in the error enum but nothing could produce
//! it: there was no way to ask for cancellation. The variant was aspirational,
//! and an aspirational cancellation contract is worse than none, because
//! callers plan around it.
//!
//! This is a plain atomic flag, not an async runtime. Geometry providers are
//! synchronous and CPU-bound; a token they poll costs one relaxed atomic load
//! at whatever granularity they declare.
//!
//! # Safety of cancellation
//!
//! Cancellation is **safe**, never partial. A provider returns either a
//! complete result or [`GeomError::Cancelled`]. It must never return a
//! half-cut mesh, because a partial solid is indistinguishable from a valid
//! one downstream and corrupts every quantity derived from it.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::{GeomError, GeomResult};

/// A shared cancellation flag.
///
/// Cloning shares the same flag, so one handle cancels work holding any clone.
#[derive(Debug, Clone, Default)]
pub struct CancellationToken {
    flag: Arc<AtomicBool>,
}

impl PartialEq for CancellationToken {
    /// Identity comparison: two handles are equal when they share one flag.
    ///
    /// Deliberately not a comparison of current cancelled state, which would
    /// make two unrelated uncancelled tokens compare equal and then diverge.
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.flag, &other.flag)
    }
}

impl Eq for CancellationToken {}

impl CancellationToken {
    /// A token that is not cancelled.
    pub fn new() -> Self {
        Self::default()
    }

    /// Request cancellation. Idempotent, and callable from any thread.
    pub fn cancel(&self) {
        self.flag.store(true, Ordering::Relaxed);
    }

    /// Whether cancellation has been requested.
    pub fn is_cancelled(&self) -> bool {
        self.flag.load(Ordering::Relaxed)
    }

    /// `Err(GeomError::Cancelled)` when cancelled, else `Ok(())`.
    ///
    /// The shape providers use at a poll point: `token.check()?`.
    pub fn check(&self) -> GeomResult<()> {
        if self.is_cancelled() {
            return Err(GeomError::Cancelled);
        }
        Ok(())
    }
}

/// How finely a provider polls its cancellation token.
///
/// Declared per provider and asserted by the conformance suite, so a caller
/// learns the real latency instead of assuming instant cancellation. An honest
/// coarse granularity beats a false fine one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CancellationGranularity {
    /// The provider never polls; cancellation has no effect once dispatched.
    ///
    /// The correct declaration for a provider wrapping an opaque routine that
    /// takes no token. It is honest, and it is what lets a caller decide to
    /// chunk the work itself.
    None,
    /// Polled between whole sub-operations, e.g. between tools in a batch.
    ///
    /// Latency is bounded by one sub-operation, not by the whole batch.
    BetweenOperations,
    /// Polled inside the algorithm's inner loops.
    Incremental,
}
