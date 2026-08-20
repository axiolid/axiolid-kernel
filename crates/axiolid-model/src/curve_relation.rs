//! Curve relationships that require graph references.

use axiolid_core::{Point2, Point3, Scalar, Vec3};

use crate::NodeId;

/// One trim selector preserved from a source representation.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TrimSelector {
    /// Curve parameter.
    Parameter(Scalar),
    /// Two-dimensional point.
    Point2(Point2),
    /// Three-dimensional point.
    Point3(Point3),
}

/// Preference when both parameter and Cartesian trim selectors exist.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TrimmingPreference {
    /// Prefer parameter values.
    Parameter,
    /// Prefer Cartesian points.
    Cartesian,
    /// Use source order when no preference was stated.
    Unspecified,
}

/// Continuity declared between consecutive composite segments.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Transition {
    /// Discontinuous.
    Discontinuous,
    /// Position continuous.
    Continuous,
    /// Position and tangent continuous.
    ContinuousSameGradient,
    /// Position, tangent, and curvature continuous.
    ContinuousSameGradientSameCurvature,
}

/// One oriented curve in a composite.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CurveSegment {
    /// Child curve.
    pub curve: NodeId,
    /// Whether child parameterization agrees with composite orientation.
    pub same_sense: bool,
    /// Transition from the preceding segment.
    pub transition: Transition,
}

/// Relationship between curve nodes.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub enum CurveRelation {
    /// Ordered composite curve.
    Composite { segments: Vec<CurveSegment> },
    /// Trimmed view of a basis curve.
    Trimmed {
        basis: NodeId,
        start: Vec<TrimSelector>,
        end: Vec<TrimSelector>,
        sense_agreement: bool,
        preference: TrimmingPreference,
    },
    /// Constant-distance offset.
    Offset {
        basis: NodeId,
        distance: Scalar,
        reference_direction: Option<Vec3>,
    },
    /// Three-dimensional curve associated with one or more surfaces/pcurves.
    SurfaceCurve {
        curve_3d: NodeId,
        associated_geometry: Vec<NodeId>,
        master: MasterRepresentation,
    },
    /// Two-dimensional parameter curve on a surface.
    ParameterCurve {
        basis_surface: NodeId,
        reference_curve: NodeId,
    },
}

/// Which representation governs a redundant surface-curve definition.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MasterRepresentation {
    /// Three-dimensional curve.
    Curve3d,
    /// Parameter curve.
    ParameterCurve,
    /// Both are authoritative and must agree.
    Both,
    /// Unspecified.
    Unspecified,
}

impl CurveRelation {
    pub(crate) fn references(&self, out: &mut Vec<NodeId>) {
        match self {
            Self::Composite { segments } => out.extend(segments.iter().map(|item| item.curve)),
            Self::Trimmed { basis, .. } | Self::Offset { basis, .. } => out.push(*basis),
            Self::SurfaceCurve {
                curve_3d,
                associated_geometry,
                ..
            } => {
                out.push(*curve_3d);
                out.extend(associated_geometry.iter().copied());
            }
            Self::ParameterCurve {
                basis_surface,
                reference_curve,
            } => out.extend([*basis_surface, *reference_curve]),
        }
    }
}
