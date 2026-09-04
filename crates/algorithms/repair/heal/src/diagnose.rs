//! Producing a [`Diagnosis`] from a mesh (#73).
//!
//! # Why this exists
//!
//! `Diagnosis` and `DefectKind` were vocabulary: complete, well-named, and
//! produced by nothing. `audit_mesh` counts defects but returns only counts,
//! so a caller learned *how many* boundary edges a mesh had and never *which*.
//! A repair cannot act on a count, and a report that cannot name what it
//! found is not auditable.
//!
//! # Counts versus locations
//!
//! Where the audit knows a location it is recorded in `Defect::element`;
//! where it knows only a count, the defect carries the count in `detail` and
//! leaves `element` empty rather than inventing an index. Self-intersection
//! is the one class that always has a location, because it is computed here
//! rather than counted by the audit.

use axiolid_core::Tolerance;
use axiolid_mesh::{audit_mesh, TriangleMeshView};

use crate::diagnosis::{Defect, DefectKind, Diagnosis};
use crate::intersect::self_intersections;

/// Diagnose a triangle mesh, reporting every defect class it exhibits.
///
/// Defects are returned in a deterministic order: structural classes in the
/// audit's own order first, then self-intersecting pairs in sorted order. The
/// result is empty exactly when the mesh is clean.
#[must_use]
pub fn diagnose<M: TriangleMeshView + ?Sized>(mesh: &M, tolerance: Tolerance) -> Diagnosis {
    let health = audit_mesh(mesh, tolerance);
    let mut defects = Vec::new();

    if health.non_manifold_edges != 0 {
        defects.push(counted(
            DefectKind::NonManifoldEdge,
            health.non_manifold_edges,
            "edges used by other than two faces",
        ));
    }
    if health.inconsistent_winding_edges != 0 {
        defects.push(counted(
            DefectKind::InconsistentOrientation,
            health.inconsistent_winding_edges,
            "edges whose adjacent faces disagree on direction",
        ));
    }
    if health.boundary_edges != 0 {
        defects.push(counted(
            DefectKind::OpenShell,
            health.boundary_edges,
            "edges used by exactly one face",
        ));
    }
    if health.degenerate_triangles != 0 {
        defects.push(counted(
            DefectKind::DegenerateElement,
            health.degenerate_triangles,
            "triangles with zero area",
        ));
    }

    for pair in self_intersections(mesh) {
        defects.push(Defect {
            kind: DefectKind::SelfIntersection,
            element: Some(pair.first),
            detail: Some(format!("crosses triangle {}", pair.second)),
        });
    }

    Diagnosis { defects }
}

/// A defect the audit can count but not locate.
fn counted(kind: DefectKind, count: usize, what: &str) -> Defect {
    Defect {
        kind,
        element: None,
        detail: Some(format!("{count} {what}")),
    }
}
