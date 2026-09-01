//! Public policy and certificate values for exhaustive projection.

use axiolid_contracts::{GeomError, GeomResult};
use axiolid_core::{Point2, Point3, Scalar, Tolerance};

/// Closed native-parameter interval that may contain a global minimizer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParameterInterval {
    /// Inclusive lower parameter.
    pub start: Scalar,
    /// Inclusive upper parameter.
    pub end: Scalar,
}

/// Explicit work and accuracy policy for globally certified projection.
///
/// Certification succeeds only when the returned global distance-bound gap is
/// no larger than `tolerance.linear()`. `max_nodes` independently caps both
/// pre-search refinement work and generated subdivision cells; pair queries
/// share one refinement allowance across both input curves. One refinement
/// work unit is one newly allocated homogeneous-control, expanded-knot, emitted
/// control, or cell slot. Binary subdivision is also depth-bounded. There is
/// deliberately no context-free `Default`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CertifiedProjectionOptions {
    tolerance: Tolerance,
    max_nodes: u32,
    max_depth: u16,
}

impl CertifiedProjectionOptions {
    /// Construct a non-vacuous certification policy.
    pub fn new(tolerance: Tolerance, max_nodes: u32, max_depth: u16) -> GeomResult<Self> {
        if max_nodes == 0 || max_depth == 0 {
            return Err(GeomError::InvalidInput(
                "certified projection budgets must be non-zero".to_owned(),
            ));
        }
        Ok(Self {
            tolerance,
            max_nodes,
            max_depth,
        })
    }

    /// Required upper-minus-lower global distance gap.
    pub const fn tolerance(self) -> Tolerance {
        self.tolerance
    }

    /// Maximum refinement-allocation work units per query phase and maximum
    /// number of generated search cells. Pair queries share the refinement
    /// allowance across both curves.
    pub const fn max_nodes(self) -> u32 {
        self.max_nodes
    }

    /// Maximum binary subdivision depth of one Bézier segment.
    pub const fn max_depth(self) -> u16 {
        self.max_depth
    }
}

/// Maximum accepted shared work budget for certified surface projection.
pub const MAX_CERTIFIED_SURFACE_PROJECTION_WORK: u32 = 100_000;

/// Maximum accepted binary subdivision depth for certified surface projection.
pub const MAX_CERTIFIED_SURFACE_PROJECTION_DEPTH: u16 = 64;

/// Explicit accuracy and work policy for globally certified surface projection.
///
/// `distance_tolerance` bounds the certified model-space distance gap.
/// `parameter_tolerance` independently bounds both native parameter widths;
/// native parameters are not assumed to have model-space units. `max_work`
/// is one shared cap for Bézier conversion, generated search cells, root and
/// child patch bounds, patch restriction, and representative enclosure.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CertifiedSurfaceProjectionOptions {
    distance_tolerance: Tolerance,
    parameter_tolerance: Scalar,
    max_work: u32,
    max_depth: u16,
}

impl CertifiedSurfaceProjectionOptions {
    /// Construct a non-vacuous, hard-capped certification policy.
    pub fn new(
        distance_tolerance: Tolerance,
        parameter_tolerance: Scalar,
        max_work: u32,
        max_depth: u16,
    ) -> GeomResult<Self> {
        if !parameter_tolerance.is_finite() || parameter_tolerance <= 0.0 {
            return Err(GeomError::InvalidInput(
                "surface projection parameter tolerance must be positive and finite".to_owned(),
            ));
        }
        if max_work == 0
            || max_work > MAX_CERTIFIED_SURFACE_PROJECTION_WORK
            || max_depth == 0
            || max_depth > MAX_CERTIFIED_SURFACE_PROJECTION_DEPTH
        {
            return Err(GeomError::InvalidInput(format!(
                "surface projection budgets must be in 1..={MAX_CERTIFIED_SURFACE_PROJECTION_WORK} work units and 1..={MAX_CERTIFIED_SURFACE_PROJECTION_DEPTH} depth"
            )));
        }
        Ok(Self {
            distance_tolerance,
            parameter_tolerance,
            max_work,
            max_depth,
        })
    }

    /// Required outward upper-minus-lower global distance gap.
    pub const fn distance_tolerance(self) -> Tolerance {
        self.distance_tolerance
    }

    /// Required maximum width of each retained native parameter interval.
    pub const fn parameter_tolerance(self) -> Scalar {
        self.parameter_tolerance
    }

    /// Shared conversion, search, restriction, and bound-construction work cap.
    pub const fn max_work(self) -> u32 {
        self.max_work
    }

    /// Maximum binary subdivision depth of one root Bézier patch.
    pub const fn max_depth(self) -> u16 {
        self.max_depth
    }
}

/// Closed native `(u, v)` box that may contain a global minimizer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SurfaceParameterBox {
    /// Closed native U interval.
    pub u: ParameterInterval,
    /// Closed native V interval.
    pub v: ParameterInterval,
}

/// Why a valid surface projection remains unresolved without exhausting work.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceProjectionUnresolvedReason {
    /// At least one surviving candidate reached the configured depth cap.
    DepthLimit,
    /// Binary64 midpoint subdivision no longer advances on surviving candidates.
    FloatingPointNoProgress,
}

/// Globally bounded closest-point certificate for a spatial surface.
///
/// The representative is a deterministic scalar-oracle evaluation. Its exact
/// binary64 `(u, v)` parameters are also interval-evaluated, and that enclosure
/// provides `distance_upper_bound`; the scalar `distance` is descriptive and
/// is not substituted for the outward certificate bound.
#[derive(Debug, Clone, PartialEq)]
pub struct SurfaceProjectionCertificate3 {
    /// Native U parameter of the attained representative.
    pub u: Scalar,
    /// Native V parameter of the attained representative.
    pub v: Scalar,
    /// Scalar-oracle approximation of the surface point at `(u, v)`.
    pub point: Point3,
    /// Euclidean distance of the scalar-oracle representative.
    pub distance: Scalar,
    /// Outward lower bound on the global minimum distance.
    pub distance_lower_bound: Scalar,
    /// Outward upper bound from evaluating the exact representative parameters.
    pub distance_upper_bound: Scalar,
    /// Closed native boxes whose union contains every global minimizer.
    pub possible_minimizer_boxes: Vec<SurfaceParameterBox>,
    /// Number of generated search cells, including root patches.
    pub visited_nodes: u32,
}

impl SurfaceProjectionCertificate3 {
    /// Outward-rounded certified global distance gap.
    pub fn gap(&self) -> Scalar {
        crate::certified_bezier::next_up(
            (self.distance_upper_bound - self.distance_lower_bound).max(0.0),
        )
    }
}

/// Exhaustive bounded outcome for spatial surface projection.
#[derive(Debug, Clone, PartialEq)]
pub enum CertifiedSurfaceProjection3 {
    /// All global-distance and native-parameter proof obligations were met.
    Complete(SurfaceProjectionCertificate3),
    /// Bounds and candidates remain sound, but the configured depth or floating
    /// point parameter resolution prevented completion.
    Unresolved {
        /// Retained partial global certificate.
        certificate: SurfaceProjectionCertificate3,
        /// Exact reason completion stopped.
        reason: SurfaceProjectionUnresolvedReason,
    },
}

/// Globally bounded closest-point result for a planar curve.
#[derive(Debug, Clone, PartialEq)]
pub struct CurveProjectionCertificate2 {
    /// Native parameter of the attained representative candidate.
    pub parameter: Scalar,
    /// Scalar-oracle evaluation at `parameter`.
    pub point: Point2,
    /// Euclidean distance of the representative scalar evaluation.
    pub distance: Scalar,
    /// Conservative lower bound on the global minimum distance.
    pub distance_lower_bound: Scalar,
    /// Conservative upper bound attained by the candidate enclosure.
    pub distance_upper_bound: Scalar,
    /// Parameter cells not excluded from containing another global minimizer.
    pub possible_minimizer_intervals: Vec<ParameterInterval>,
    /// Number of generated Bézier subdivision cells.
    pub visited_nodes: u32,
}

impl CurveProjectionCertificate2 {
    /// Certified global upper-minus-lower distance gap.
    pub fn gap(&self) -> Scalar {
        (self.distance_upper_bound - self.distance_lower_bound).max(0.0)
    }
}

/// Globally bounded closest-point result for a spatial curve.
#[derive(Debug, Clone, PartialEq)]
pub struct CurveProjectionCertificate3 {
    /// Native parameter of the attained representative candidate.
    pub parameter: Scalar,
    /// Scalar-oracle evaluation at `parameter`.
    pub point: Point3,
    /// Euclidean distance of the representative scalar evaluation.
    pub distance: Scalar,
    /// Conservative lower bound on the global minimum distance.
    pub distance_lower_bound: Scalar,
    /// Conservative upper bound attained by the candidate enclosure.
    pub distance_upper_bound: Scalar,
    /// Parameter cells not excluded from containing another global minimizer.
    pub possible_minimizer_intervals: Vec<ParameterInterval>,
    /// Number of generated Bézier subdivision cells.
    pub visited_nodes: u32,
}

impl CurveProjectionCertificate3 {
    /// Certified global upper-minus-lower distance gap.
    pub fn gap(&self) -> Scalar {
        (self.distance_upper_bound - self.distance_lower_bound).max(0.0)
    }
}

/// Product-domain box that may contain a globally closest curve pair.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CurvePairParameterBox {
    /// Parameter interval on the first curve.
    pub first: ParameterInterval,
    /// Parameter interval on the second curve.
    pub second: ParameterInterval,
}

/// Globally bounded minimum-distance result for two planar curves.
#[derive(Debug, Clone, PartialEq)]
pub struct CurveDistanceCertificate2 {
    /// Representative parameter on the first curve.
    pub first_parameter: Scalar,
    /// Representative parameter on the second curve.
    pub second_parameter: Scalar,
    /// Scalar-oracle point on the first curve.
    pub first_point: Point2,
    /// Scalar-oracle point on the second curve.
    pub second_point: Point2,
    /// Euclidean distance between representative scalar evaluations.
    pub distance: Scalar,
    /// Conservative lower bound on the global minimum distance.
    pub distance_lower_bound: Scalar,
    /// Conservative upper bound attained by a candidate enclosure.
    pub distance_upper_bound: Scalar,
    /// Product cells not excluded from containing another global minimizer.
    pub possible_minimizer_boxes: Vec<CurvePairParameterBox>,
    /// Number of generated product cells.
    pub visited_nodes: u32,
}

impl CurveDistanceCertificate2 {
    /// Certified global upper-minus-lower distance gap.
    pub fn gap(&self) -> Scalar {
        (self.distance_upper_bound - self.distance_lower_bound).max(0.0)
    }
}

/// Globally bounded minimum-distance result for two spatial curves.
#[derive(Debug, Clone, PartialEq)]
pub struct CurveDistanceCertificate3 {
    /// Representative parameter on the first curve.
    pub first_parameter: Scalar,
    /// Representative parameter on the second curve.
    pub second_parameter: Scalar,
    /// Scalar-oracle point on the first curve.
    pub first_point: Point3,
    /// Scalar-oracle point on the second curve.
    pub second_point: Point3,
    /// Euclidean distance between representative scalar evaluations.
    pub distance: Scalar,
    /// Conservative lower bound on the global minimum distance.
    pub distance_lower_bound: Scalar,
    /// Conservative upper bound attained by a candidate enclosure.
    pub distance_upper_bound: Scalar,
    /// Product cells not excluded from containing another global minimizer.
    pub possible_minimizer_boxes: Vec<CurvePairParameterBox>,
    /// Number of generated product cells.
    pub visited_nodes: u32,
}

impl CurveDistanceCertificate3 {
    /// Certified global upper-minus-lower distance gap.
    pub fn gap(&self) -> Scalar {
        (self.distance_upper_bound - self.distance_lower_bound).max(0.0)
    }
}
