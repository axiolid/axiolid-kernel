//! `audit_brep` must catch structural defects, not merely describe them.
//!
//! Each test builds a B-rep with exactly one injected fault and asserts the
//! audit names it. A cube that passes proves nothing on its own: the point
//! is that a broken cube fails.

use axiolid_topology::{
    audit_brep, BRep, Edge, EdgeUse, Face, FaceBound, Loop, Orientation, Shell, Solid, Vertex,
};

/// A square face on four vertices, as one closed loop.
fn square() -> BRep<u32> {
    let mut b: BRep<u32> = BRep::default();
    let v: Vec<_> = (0..4)
        .map(|_| {
            b.add_vertex(Vertex {
                position: axiolid_core::Point3::ZERO,
            })
        })
        .collect();
    let e: Vec<_> = (0..4)
        .map(|i| {
            b.add_edge(Edge {
                start: v[i],
                end: v[(i + 1) % 4],
                curve: None,
            })
        })
        .collect();
    let lp = b.add_loop(Loop {
        edges: e
            .iter()
            .map(|&edge| EdgeUse {
                edge,
                orientation: Orientation::Forward,
            })
            .collect(),
    });
    let f = b.add_face(Face {
        surface: None,
        bounds: vec![FaceBound {
            loop_id: lp,
            orientation: Orientation::Forward,
            outer: true,
        }],
        orientation: Orientation::Forward,
    });
    let sh = b.add_shell(Shell {
        faces: vec![(f, Orientation::Forward)],
        closed: false,
    });
    b.add_solid(Solid {
        outer: sh,
        voids: Vec::new(),
    });
    b
}

#[test]
fn a_sound_square_passes() {
    let health = audit_brep(&square());
    assert!(health.is_tessellable(), "sound square: {health:?}");
    assert_eq!(health.dangling_references, 0);
    assert_eq!(health.open_loops, 0);
}

/// An open shell is legitimate: a single face bounds no volume, and the
/// audit must not pretend otherwise or reject it.
#[test]
fn a_single_face_is_tessellable_but_not_closed() {
    let health = audit_brep(&square());
    assert!(health.is_tessellable());
    assert!(
        !health.is_closed_manifold(),
        "one face cannot bound a volume: {health:?}"
    );
    assert_eq!(health.unpaired_edge_uses, 4, "every edge is a boundary");
}

/// A loop whose edges do not meet is not a boundary.
#[test]
fn a_disconnected_loop_is_reported_open() {
    let mut b: BRep<u32> = BRep::default();
    let v: Vec<_> = (0..4)
        .map(|_| {
            b.add_vertex(Vertex {
                position: axiolid_core::Point3::ZERO,
            })
        })
        .collect();
    // Two edges that share no vertex: 0->1 and 2->3.
    let e0 = b.add_edge(Edge {
        start: v[0],
        end: v[1],
        curve: None,
    });
    let e1 = b.add_edge(Edge {
        start: v[2],
        end: v[3],
        curve: None,
    });
    b.add_loop(Loop {
        edges: vec![
            EdgeUse {
                edge: e0,
                orientation: Orientation::Forward,
            },
            EdgeUse {
                edge: e1,
                orientation: Orientation::Forward,
            },
        ],
    });
    let health = audit_brep(&b);
    assert_eq!(health.open_loops, 1, "gap must be caught: {health:?}");
    assert!(!health.is_tessellable());
}

/// A shell claiming closure it does not have is the dangerous case: every
/// downstream volume and containment result silently trusts that flag.
#[test]
fn a_false_closure_claim_is_caught() {
    let mut b: BRep<u32> = BRep::default();
    let v: Vec<_> = (0..4)
        .map(|_| {
            b.add_vertex(Vertex {
                position: axiolid_core::Point3::ZERO,
            })
        })
        .collect();
    let e: Vec<_> = (0..4)
        .map(|i| {
            b.add_edge(Edge {
                start: v[i],
                end: v[(i + 1) % 4],
                curve: None,
            })
        })
        .collect();
    let lp = b.add_loop(Loop {
        edges: e
            .iter()
            .map(|&edge| EdgeUse {
                edge,
                orientation: Orientation::Forward,
            })
            .collect(),
    });
    let f = b.add_face(Face {
        surface: None,
        bounds: vec![FaceBound {
            loop_id: lp,
            orientation: Orientation::Forward,
            outer: true,
        }],
        orientation: Orientation::Forward,
    });
    b.add_shell(Shell {
        faces: vec![(f, Orientation::Forward)],
        closed: true,
    });
    let health = audit_brep(&b);
    assert_eq!(
        health.false_closure_claims, 1,
        "a lone face cannot be a closed shell: {health:?}"
    );
    assert!(!health.is_closed_manifold());
}
