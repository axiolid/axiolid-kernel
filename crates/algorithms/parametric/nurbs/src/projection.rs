//! Explicit policy and honest outcomes for bounded inverse queries.

use axiolid_contracts::{GeomError, GeomResult};
use axiolid_core::{Point2, Point3, Scalar, Tolerance};

#[derive(Debug, Clone, Copy, PartialEq)]
/// Explicit work and convergence policy for bounded projection.
///
/// The search samples every active knot span, then runs Newton refinement from
/// each start until one of these budgets is reached. There is deliberately no
/// context-free `Default`.
pub struct ProjectionOptions {
    tolerance: Tolerance,
    samples_per_span: u16,
    max_iterations: u16,
    max_starts: u32,
}

impl ProjectionOptions {
    /// Construct a non-vacuous projection policy.
    ///
    /// Returns an error when any budget is zero.
    pub fn new(
        tolerance: Tolerance,
        samples_per_span: u16,
        max_iterations: u16,
        max_starts: u32,
    ) -> GeomResult<Self> {
        if samples_per_span == 0 || max_iterations == 0 || max_starts == 0 {
            return Err(GeomError::InvalidInput(
                "projection budgets must be non-zero".to_owned(),
            ));
        }
        Ok(Self {
            tolerance,
            samples_per_span,
            max_iterations,
            max_starts,
        })
    }
    /// Linear/angular convergence tolerance.
    pub const fn tolerance(self) -> Tolerance {
        self.tolerance
    }
    /// Uniform subdivisions used to seed each active knot span.
    pub const fn samples_per_span(self) -> u16 {
        self.samples_per_span
    }
    /// Maximum Newton updates for one start.
    pub const fn max_iterations(self) -> u16 {
        self.max_iterations
    }
    /// Aggregate start limit across all spans (or span pairs).
    pub const fn max_starts(self) -> u32 {
        self.max_starts
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Outcome of the selected bounded local solve.
///
/// This status does not certify global optimality.
pub enum ProjectionStatus {
    /// First-order stationarity or tolerance-sized movement was reached.
    Converged,
    /// The selected candidate remained best when its iteration budget ended.
    BudgetExhausted,
}

#[derive(Debug, Clone, Copy, PartialEq)]
/// Best planar-curve projection candidate found within the supplied budget.
pub struct CurveProjection2 {
    /// Native curve parameter.
    pub parameter: Scalar,
    /// Evaluated point at `parameter`.
    pub point: Point2,
    /// Euclidean distance from the target.
    pub distance: Scalar,
    /// Newton updates used by the selected start.
    pub iterations: u16,
    /// Whether the selected parameter tuple touches its active domain boundary.
    pub on_boundary: bool,
    /// Local-solve outcome; not a global-optimum certificate.
    pub status: ProjectionStatus,
}

#[derive(Debug, Clone, Copy, PartialEq)]
/// Best spatial-curve projection candidate found within the supplied budget.
pub struct CurveProjection3 {
    /// Native curve parameter.
    pub parameter: Scalar,
    /// Evaluated point at `parameter`.
    pub point: Point3,
    /// Euclidean distance from the target.
    pub distance: Scalar,
    /// Newton updates used by the selected start.
    pub iterations: u16,
    /// Whether the selected parameter tuple touches its active domain boundary.
    pub on_boundary: bool,
    /// Local-solve outcome; not a global-optimum certificate.
    pub status: ProjectionStatus,
}

#[derive(Debug, Clone, Copy, PartialEq)]
/// Best surface projection candidate found within the supplied budget.
pub struct SurfaceProjection {
    /// Native first-axis surface parameter.
    pub u: Scalar,
    /// Native second-axis surface parameter.
    pub v: Scalar,
    /// Evaluated surface point at `(u, v)`.
    pub point: Point3,
    /// Euclidean distance from the target.
    pub distance: Scalar,
    /// Newton updates used by the selected start.
    pub iterations: u16,
    /// Whether the selected parameter tuple touches its active domain boundary.
    pub on_boundary: bool,
    /// Local-solve outcome; not a global-optimum certificate.
    pub status: ProjectionStatus,
}
