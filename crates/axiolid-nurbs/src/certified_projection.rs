//! Public policy and certificate values for exhaustive projection.

use axiolid_core::{Point2, Point3, Scalar, Tolerance};
use axiolid_kernel::{GeomError, GeomResult};

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
