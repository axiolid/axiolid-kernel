//! Structured geometry failures suitable for fallback and diagnostics.

use thiserror::Error;

use crate::{BackendId, Operation};

/// Result alias for geometry operations.
pub type GeomResult<T> = Result<T, GeomError>;

/// Why a backend-neutral geometry operation failed.
#[non_exhaustive]
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum GeomError {
    /// Input violates the operation's structural preconditions.
    #[error("invalid geometry input: {0}")]
    InvalidInput(String),
    /// Backend cannot implement this capability.
    #[error("backend `{backend}` does not support {operation:?}")]
    Unsupported {
        /// Backend.
        backend: BackendId,
        /// Missing capability.
        operation: Operation,
    },
    /// Backend supports the operation but not this input family.
    #[error("backend `{backend}` cannot apply {operation:?} to `{input}`")]
    UnsupportedInput {
        /// Backend.
        backend: BackendId,
        /// Requested operation.
        operation: Operation,
        /// Stable, human-readable input family.
        input: &'static str,
    },
    /// Backend exists but its hardware/runtime is unavailable.
    #[error("backend `{backend}` is unavailable: {reason}")]
    Unavailable {
        /// Backend.
        backend: BackendId,
        /// Diagnostic reason.
        reason: String,
    },
    /// Backend returned a result that violates its operation contract.
    #[error("backend `{backend}` violated its contract: {detail}")]
    BackendContractViolation {
        /// Backend that violated the contract.
        backend: BackendId,
        /// Actionable contract diagnostic.
        detail: String,
    },
    /// Dirty topology violates a manifold precondition.
    #[error("input is not manifold: {0}")]
    NotManifold(String),
    /// Numerical configuration could not be resolved reliably.
    #[error("numerically degenerate input: {0}")]
    Degenerate(String),
    /// Explicit operation budget was exceeded.
    #[error("operation exceeded its {resource} budget")]
    BudgetExceeded {
        /// Resource name, e.g. memory or iterations.
        resource: &'static str,
    },
    /// Cooperative cancellation.
    #[error("operation cancelled")]
    Cancelled,
}
