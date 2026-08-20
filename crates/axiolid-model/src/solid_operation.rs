//! Solid construction relationships and CSG instructions.

use axiolid_core::{BooleanOperator, Point3, Scalar, Transform3, Vec3};

use crate::NodeId;

/// Position of one section along a sectioned sweep.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Section {
    /// Profile node.
    pub profile: NodeId,
    /// Local placement of the profile.
    pub placement: Transform3,
}

/// Relationship that constructs a solid from lower-level geometry.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub enum SolidOperation {
    /// Linear extrusion of a profile.
    Extrusion {
        profile: NodeId,
        direction: Vec3,
        depth: Scalar,
    },
    /// Tapered linear extrusion between two profiles.
    TaperedExtrusion {
        start_profile: NodeId,
        end_profile: NodeId,
        direction: Vec3,
        depth: Scalar,
    },
    /// Revolution of a profile.
    Revolution {
        profile: NodeId,
        axis_origin: Point3,
        axis_direction: Vec3,
        angle: Scalar,
    },
    /// Tapered revolution between two profiles.
    TaperedRevolution {
        start_profile: NodeId,
        end_profile: NodeId,
        axis_origin: Point3,
        axis_direction: Vec3,
        angle: Scalar,
    },
    /// Disk swept along a directrix curve.
    SweptDisk {
        directrix: NodeId,
        radius: Scalar,
        inner_radius: Option<Scalar>,
        parameter_range: Option<(Scalar, Scalar)>,
    },
    /// Profile swept along a directrix using a fixed reference direction.
    FixedReferenceSweep {
        profile: NodeId,
        directrix: NodeId,
        reference_direction: Vec3,
        parameter_range: Option<(Scalar, Scalar)>,
    },
    /// Profile swept along a directrix constrained by a reference surface.
    SurfaceCurveSweep {
        profile: NodeId,
        directrix: NodeId,
        reference_surface: NodeId,
        parameter_range: Option<(Scalar, Scalar)>,
    },
    /// Sections interpolated along a spine.
    SectionedSpine {
        spine: NodeId,
        sections: Vec<Section>,
    },
    /// General CSG binary operation.
    Boolean {
        left: NodeId,
        right: NodeId,
        operator: BooleanOperator,
    },
    /// Unbounded half-space clipped by a finite boundary geometry.
    BoundedHalfSpace {
        half_space: NodeId,
        boundary: NodeId,
        placement: Transform3,
    },
}

impl SolidOperation {
    pub(crate) fn references(&self, out: &mut Vec<NodeId>) {
        match self {
            Self::Extrusion { profile, .. } | Self::Revolution { profile, .. } => {
                out.push(*profile)
            }
            Self::TaperedExtrusion {
                start_profile,
                end_profile,
                ..
            }
            | Self::TaperedRevolution {
                start_profile,
                end_profile,
                ..
            } => out.extend([*start_profile, *end_profile]),
            Self::SweptDisk { directrix, .. } => out.push(*directrix),
            Self::FixedReferenceSweep {
                profile, directrix, ..
            } => out.extend([*profile, *directrix]),
            Self::SurfaceCurveSweep {
                profile,
                directrix,
                reference_surface,
                ..
            } => out.extend([*profile, *directrix, *reference_surface]),
            Self::SectionedSpine { spine, sections } => {
                out.push(*spine);
                out.extend(sections.iter().map(|section| section.profile));
            }
            Self::Boolean { left, right, .. } => out.extend([*left, *right]),
            Self::BoundedHalfSpace {
                half_space,
                boundary,
                ..
            } => out.extend([*half_space, *boundary]),
        }
    }
}
