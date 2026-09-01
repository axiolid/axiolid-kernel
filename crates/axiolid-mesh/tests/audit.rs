use axiolid_core::{Point3, Tolerance};
use axiolid_mesh::{audit_mesh, audit_mesh_scratch_bytes, try_audit_mesh, TriangleMeshView};

#[derive(Debug)]
struct ForeignMesh {
    positions: Vec<Point3>,
    triangles: Vec<[u64; 3]>,
}

impl TriangleMeshView for ForeignMesh {
    fn position_count(&self) -> usize {
        self.positions.len()
    }

    fn position(&self, index: usize) -> Point3 {
        self.positions[index]
    }

    fn triangle_count(&self) -> usize {
        self.triangles.len()
    }

    fn triangle(&self, index: usize) -> [u64; 3] {
        self.triangles[index]
    }
}

#[test]
fn audit_keeps_dirty_meshes_observable_and_deterministic() {
    let mesh = ForeignMesh {
        positions: vec![
            Point3::ZERO,
            Point3::X,
            Point3::Y,
            Point3::new(f64::NAN, 0.0, 0.0),
        ],
        triangles: vec![[0, 1, 2], [0, 2, 9], [0, 1, 1], [0, 1, 3]],
    };

    let report = audit_mesh(&mesh, Tolerance::ZERO);

    assert_eq!(report.positions, 4);
    assert_eq!(report.triangles, 4);
    assert_eq!(report.usable_triangles, 1);
    assert_eq!(report.invalid_indices, 1);
    assert_eq!(report.non_finite_positions, 1);
    assert_eq!(report.degenerate_triangles, 1);
    assert_eq!(report.boundary_edges, 3);
    assert_eq!(report.non_manifold_edges, 0);
    assert_eq!(report.first_invalid_index, Some((1, 9)));
    assert_eq!(report.first_non_finite_position, Some(3));
    assert!(!report.is_surface_usable());
    assert!(!report.is_closed_two_manifold());
}

#[test]
fn degeneracy_respects_the_explicit_linear_tolerance() {
    let mesh = ForeignMesh {
        positions: vec![Point3::ZERO, Point3::X, Point3::new(0.0, 1e-13, 0.0)],
        triangles: vec![[0, 1, 2]],
    };

    assert_eq!(audit_mesh(&mesh, Tolerance::ZERO).degenerate_triangles, 0);
    assert_eq!(audit_mesh(&mesh, Tolerance::METRE).degenerate_triangles, 1);
}

#[test]
fn bounded_audit_matches_the_compatibility_audit() {
    let mesh = ForeignMesh {
        positions: vec![Point3::ZERO, Point3::X, Point3::Y, Point3::Z],
        triangles: vec![[0, 2, 1], [0, 1, 3], [1, 2, 3], [2, 0, 3]],
    };

    let bounded = try_audit_mesh(&mesh, Tolerance::ZERO).expect("bounded audit");
    assert_eq!(bounded, audit_mesh(&mesh, Tolerance::ZERO));
    assert!(bounded.is_closed_two_manifold());
    assert_eq!(audit_mesh_scratch_bytes(0), Some(0));
    assert_eq!(audit_mesh_scratch_bytes(usize::MAX), None);
}
