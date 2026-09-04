//! The reference [`Measure`] provider for triangle meshes.
//!
//! # Why this exists
//!
//! `Measure<T>` was a trait with no implementor, and `MassProperties` a struct
//! nothing produced. The parts were all present -- area, volume and centroid
//! were computed by free functions in [`crate::mesh`] -- but nothing assembled
//! them into the declared contract, and `second_moment_diagonal` was never
//! computed at all.
//!
//! # Refusal over a plausible number
//!
//! Mass properties are only defined for a closed oriented solid. An open shell
//! has a surface area but no enclosed volume, so this provider refuses it
//! rather than reporting the divergence sum of an open surface, which is a
//! finite number with no meaning.

use axiolid_core::Tolerance;
use axiolid_mesh::TriMesh;

use crate::{
    mesh::{second_moments, surface_properties, volume_properties},
    MassProperties, Measure, MeshMeasureError,
};

/// Reference mass-property provider for triangle meshes.
///
/// Deterministic: the sums run in triangle order, so the result is
/// reproducible for a given mesh.
#[derive(Debug, Default, Clone, Copy)]
pub struct MeshMeasure;

impl Measure<TriMesh> for MeshMeasure {
    type Error = MeshMeasureError;

    /// Area, signed volume, volume centroid and second moments in one pass.
    ///
    /// The centroid reported is the VOLUME centroid, not the area centroid:
    /// for mass properties of a solid that is the meaningful one, and
    /// [`surface_properties`] still exposes the area centroid separately for
    /// callers measuring a shell.
    fn measure(
        &self,
        value: &TriMesh,
        tolerance: Tolerance,
    ) -> Result<MassProperties, Self::Error> {
        // Volume first: it is the strictest admission (closed two-manifold),
        // so an unusable mesh is refused before any further work.
        let volume = volume_properties(value, tolerance)?;
        let surface = surface_properties(value, tolerance)?;
        let second_moment_diagonal = second_moments(value, tolerance)?;
        Ok(MassProperties {
            area: surface.area,
            signed_volume: volume.signed_volume,
            centroid: volume.centroid,
            second_moment_diagonal,
        })
    }
}
