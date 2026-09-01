//! Generalized winding-number accumulation over validated triangle views.
//!
//! This module reports a raw metric-like winding measure; it does not classify
//! points as inside or outside. Callers retain shell-closure requirements,
//! orientation policy, classification thresholds, and application semantics.

use core::fmt;

use axiolid_core::{Point3, Tolerance};
use axiolid_mesh::{audit_mesh, MeshHealth, TriangleMeshView};

/// Raw winding accumulation for one query point.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WindingNumber {
    /// Sum of oriented triangle solid angles divided by `4π`.
    pub value: f64,
    /// Faces ignored because the query point was within the supplied linear
    /// tolerance of one of their vertices, where their solid angle is undefined.
    pub skipped_singular_triangles: usize,
}

/// Failure to prepare or evaluate a winding query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WindingError {
    /// The mesh contains invalid indices, non-finite coordinates, no usable
    /// triangles, or degenerate triangles under the supplied tolerance.
    MeshNotWindingUsable(MeshHealth),
    /// Winding is undefined for a non-finite query point.
    NonFinitePoint,
}

impl fmt::Display for WindingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MeshNotWindingUsable(_) => {
                formatter.write_str("mesh is not usable for winding accumulation")
            }
            Self::NonFinitePoint => formatter.write_str("winding query point must be finite"),
        }
    }
}

impl std::error::Error for WindingError {}

/// A read-only, structurally prepared mesh for repeated winding queries.
///
/// Preparation performs the O(triangles) structural audit once. Individual
/// queries only read caller-owned geometry and have no global state, mutation,
/// threading assumption, or source identity. A batch/provider API can reuse
/// this exact prepared-input contract later.
#[derive(Clone, Copy)]
pub struct WindingMesh<'a, M: TriangleMeshView + ?Sized> {
    mesh: &'a M,
    tolerance: Tolerance,
}

impl<M: TriangleMeshView + ?Sized> fmt::Debug for WindingMesh<'_, M> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("WindingMesh")
            .field("positions", &self.mesh.position_count())
            .field("triangles", &self.mesh.triangle_count())
            .field("tolerance", &self.tolerance)
            .finish()
    }
}

impl<'a, M: TriangleMeshView + ?Sized> WindingMesh<'a, M> {
    /// Validate and prepare a foreign or owned triangle mesh.
    ///
    /// Closedness is intentionally not required: generalized winding returns a
    /// continuous raw value for open surfaces too. Degenerate faces are rejected
    /// because their solid angles are not defined by this operation.
    pub fn prepare(mesh: &'a M, tolerance: Tolerance) -> Result<Self, WindingError> {
        let health = audit_mesh(mesh, tolerance);
        if !health.is_surface_usable() || health.degenerate_triangles != 0 {
            return Err(WindingError::MeshNotWindingUsable(health));
        }
        Ok(Self { mesh, tolerance })
    }

    /// Accumulate the generalized winding number at one finite query point.
    pub fn winding_number(&self, point: Point3) -> Result<WindingNumber, WindingError> {
        if !point.is_finite() {
            return Err(WindingError::NonFinitePoint);
        }

        let mut solid_angle = 0.0;
        let mut skipped_singular_triangles = 0;
        let singular_distance_squared = self.tolerance.linear().powi(2);

        for triangle_index in 0..self.mesh.triangle_count() {
            let indices = self
                .mesh
                .triangle(triangle_index)
                .map(|index| index as usize);
            let [a, b, c] = indices.map(|index| self.mesh.position(index) - point);
            let squared_lengths = [a.length_squared(), b.length_squared(), c.length_squared()];
            let singular = squared_lengths.iter().any(|&squared_length| {
                squared_length == 0.0
                    || (self.tolerance.linear() > 0.0 && squared_length < singular_distance_squared)
            });
            if singular {
                skipped_singular_triangles += 1;
                continue;
            }

            let [length_a, length_b, length_c] = squared_lengths.map(f64::sqrt);
            let numerator = a.dot(b.cross(c));
            let denominator = length_a * length_b * length_c
                + a.dot(b) * length_c
                + b.dot(c) * length_a
                + c.dot(a) * length_b;
            solid_angle += 2.0 * numerator.atan2(denominator);
        }

        Ok(WindingNumber {
            value: solid_angle / (4.0 * std::f64::consts::PI),
            skipped_singular_triangles,
        })
    }
}
