//! Deterministic raw triangle-mesh measures.
use axiolid_core::{Point3, Tolerance};
use axiolid_mesh::{audit_mesh, MeshHealth, TriangleMeshView};
use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SurfaceProperties {
    pub area: f64,
    pub centroid: Point3,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VolumeProperties {
    pub signed_volume: f64,
    pub centroid: Point3,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MeshMeasureError {
    MeshNotSurfaceUsable(MeshHealth),
    MeshNotVolumeUsable(MeshHealth),
    ZeroSurfaceArea,
    ZeroSignedVolume,
}
impl fmt::Display for MeshMeasureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("mesh is not suitable for requested raw measure")
    }
}
impl std::error::Error for MeshMeasureError {}

pub fn surface_properties<M: TriangleMeshView + ?Sized>(
    mesh: &M,
    tolerance: Tolerance,
) -> Result<SurfaceProperties, MeshMeasureError> {
    let health = audit_mesh(mesh, tolerance);
    if !health.is_surface_usable() || health.degenerate_triangles != 0 {
        return Err(MeshMeasureError::MeshNotSurfaceUsable(health));
    }
    let (mut area, mut weighted) = (0.0, Point3::ZERO);
    for i in 0..mesh.triangle_count() {
        let [a, b, c] = mesh.triangle(i).map(|j| mesh.position(j as usize));
        let weight = (b - a).cross(c - a).length() * 0.5;
        area += weight;
        weighted += (a + b + c) * (weight / 3.0);
    }
    if !area.is_finite() || area == 0.0 {
        return Err(MeshMeasureError::ZeroSurfaceArea);
    }
    Ok(SurfaceProperties {
        area,
        centroid: weighted / area,
    })
}

pub fn volume_properties<M: TriangleMeshView + ?Sized>(
    mesh: &M,
    tolerance: Tolerance,
) -> Result<VolumeProperties, MeshMeasureError> {
    let health = audit_mesh(mesh, tolerance);
    if !health.is_closed_two_manifold() {
        return Err(MeshMeasureError::MeshNotVolumeUsable(health));
    }
    let (mut volume, mut weighted) = (0.0, Point3::ZERO);
    for i in 0..mesh.triangle_count() {
        let [a, b, c] = mesh.triangle(i).map(|j| mesh.position(j as usize));
        let tetra = a.dot(b.cross(c)) / 6.0;
        volume += tetra;
        weighted += (a + b + c) * (tetra / 4.0);
    }
    if !volume.is_finite() || volume == 0.0 {
        return Err(MeshMeasureError::ZeroSignedVolume);
    }
    Ok(VolumeProperties {
        signed_volume: volume,
        centroid: weighted / volume,
    })
}
