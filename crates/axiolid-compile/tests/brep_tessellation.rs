//! Faceted B-rep tessellation gates.

use axiolid_boolmesh::BoolmeshBoolean;
use axiolid_compile::ScalarCompiler;
use axiolid_core::Vec3;
use axiolid_kernel::{ExecutionOptions, GeometryCompiler};
use axiolid_model::{GeometryGraphBuilder, GeometryNode};
use axiolid_topology::audit_brep;
use axiolid_topology::{
    BRep, Edge, EdgeUse, Face, FaceBound, Loop, Orientation, Shell, Solid, Vertex,
};
use std::collections::HashMap;

/// Build a unit cube as topology, the way lowering produces it.
fn cube() -> BRep<axiolid_model::NodeId> {
    cube_with_sense(Orientation::Forward)
}

/// Same cube, but every face registered with the given shell sense.
fn cube_with_sense(sense: Orientation) -> BRep<axiolid_model::NodeId> {
    let (mut brep, shell) = cube_shell(sense);
    brep.add_solid(Solid {
        outer: shell,
        voids: Vec::new(),
    });
    brep
}

/// The cube's faces and shell, leaving solid registration to the caller.
fn cube_shell(sense: Orientation) -> (BRep<axiolid_model::NodeId>, axiolid_topology::ShellId) {
    let mut brep = BRep::default();
    let corners = [
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [1.0, 1.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 0.0, 1.0],
        [1.0, 0.0, 1.0],
        [1.0, 1.0, 1.0],
        [0.0, 1.0, 1.0],
    ];
    let verts: Vec<_> = corners
        .iter()
        .map(|c| {
            brep.add_vertex(Vertex {
                position: Vec3::new(c[0], c[1], c[2]),
            })
        })
        .collect();
    // Outward-facing quads (CCW seen from outside).
    let quads = [
        [0usize, 3, 2, 1],
        [4, 5, 6, 7],
        [0, 1, 5, 4],
        [1, 2, 6, 5],
        [2, 3, 7, 6],
        [3, 0, 4, 7],
    ];
    let mut edges: HashMap<(usize, usize), axiolid_topology::EdgeId> = HashMap::new();
    let mut faces = Vec::new();
    for quad in quads {
        let mut uses = Vec::new();
        for i in 0..4 {
            let (a, b) = (quad[i], quad[(i + 1) % 4]);
            let key = if a < b { (a, b) } else { (b, a) };
            let id = *edges.entry(key).or_insert_with(|| {
                brep.add_edge(Edge {
                    start: verts[key.0],
                    end: verts[key.1],
                    curve: None,
                })
            });
            let orientation = if a == key.0 {
                Orientation::Forward
            } else {
                Orientation::Reversed
            };
            uses.push(EdgeUse {
                edge: id,
                orientation,
            });
        }
        let wire = brep.add_loop(Loop { edges: uses });
        faces.push(brep.add_face(Face {
            surface: None,
            bounds: vec![FaceBound {
                loop_id: wire,
                orientation: Orientation::Forward,
                outer: true,
            }],
            orientation: Orientation::Forward,
        }));
    }
    let shell = brep.add_shell(Shell {
        faces: faces.iter().map(|f| (*f, sense)).collect(),
        closed: true,
    });
    (brep, shell)
}

fn compile(brep: BRep<axiolid_model::NodeId>) -> axiolid_mesh::TriMesh {
    let mut builder = GeometryGraphBuilder::new();
    let root = builder.push(GeometryNode::BRep(brep)).expect("push");
    let graph = builder.finish(vec![root]).expect("finish");
    let compiler = ScalarCompiler::new(BoolmeshBoolean::new());
    compiler
        .compile(
            &graph,
            root,
            &ExecutionOptions::new(axiolid_core::Tolerance::METRE),
        )
        .expect("cube compiles")
}

/// A cube tessellates to a welded, closed, outward mesh.
///
/// Per-face vertex copies would still render correctly but leave every edge
/// unshared, so the weld is asserted through edge parity rather than by
/// counting positions alone.
#[test]
fn a_cube_tessellates_to_a_closed_welded_mesh() {
    let mesh = compile(cube());

    assert_eq!(
        mesh.positions.len(),
        8,
        "one position per topological vertex"
    );
    assert_eq!(
        mesh.indices.len(),
        12 * 3,
        "six quads become twelve triangles"
    );

    let mut directed: HashMap<(u32, u32), i32> = HashMap::new();
    for t in mesh.indices.chunks_exact(3) {
        for &(a, b) in &[(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
            *directed.entry((a, b)).or_default() += 1;
        }
    }
    for (&(a, b), &count) in &directed {
        assert_eq!(count, 1, "directed edge {a}->{b} repeats");
        assert_eq!(
            directed.get(&(b, a)).copied().unwrap_or(0),
            1,
            "edge {a}-{b} unpaired"
        );
    }
}

/// Signed volume is positive: the winding survived tessellation.
///
/// A flipped face is invisible to a triangle count and to edge parity, but
/// inverts the divergence integral.
#[test]
fn tessellated_winding_stays_outward() {
    let mesh = compile(cube());
    let mut volume = 0.0;
    for t in mesh.indices.chunks_exact(3) {
        let a = mesh.positions[t[0] as usize];
        let b = mesh.positions[t[1] as usize];
        let c = mesh.positions[t[2] as usize];
        volume += a.dot(b.cross(c)) / 6.0;
    }
    assert!(
        (volume - 1.0).abs() < 1e-9,
        "unit cube volume, got {volume}"
    );
}

/// Void shells contribute no surface: they are boolean intent, not geometry.
///
/// The cavity uses its own smaller corners, so tessellating it would add
/// distinct positions and triangles rather than welding away invisibly.
#[test]
fn void_shells_do_not_add_surface() {
    let (mut brep, outer) = cube_shell(Orientation::Forward);

    // An inner cube face at 0.25..0.75 -- geometrically distinct from the shell.
    let inner: Vec<_> = [
        [0.25, 0.25, 0.25],
        [0.75, 0.25, 0.25],
        [0.75, 0.75, 0.25],
        [0.25, 0.75, 0.25],
    ]
    .iter()
    .map(|c| {
        brep.add_vertex(Vertex {
            position: Vec3::new(c[0], c[1], c[2]),
        })
    })
    .collect();
    let mut uses = Vec::new();
    for index in 0..4 {
        let edge = brep.add_edge(Edge {
            start: inner[index],
            end: inner[(index + 1) % 4],
            curve: None,
        });
        uses.push(EdgeUse {
            edge,
            orientation: Orientation::Forward,
        });
    }
    let wire = brep.add_loop(Loop { edges: uses });
    let face = brep.add_face(Face {
        surface: None,
        bounds: vec![FaceBound {
            loop_id: wire,
            orientation: Orientation::Forward,
            outer: true,
        }],
        orientation: Orientation::Forward,
    });
    let void = brep.add_shell(Shell {
        faces: vec![(face, Orientation::Forward)],
        closed: true,
    });
    // The void belongs to solids()[0], which is what tessellation reads.
    brep.add_solid(Solid {
        outer,
        voids: vec![void],
    });

    let mesh = compile(brep);
    assert_eq!(
        mesh.positions.len(),
        8,
        "void corners must not reach the mesh"
    );
    assert_eq!(
        mesh.indices.len(),
        36,
        "only the outer shell is tessellated; voids are boolean intent"
    );
}
/// A reversed shell sense flips the emitted winding.
///
/// The cube is symmetric enough that a missed flip still yields a closed
/// mesh, so orientation is measured by signed volume: inverting every face
/// negates it.
#[test]
fn a_reversed_shell_inverts_the_signed_volume() {
    let mesh = compile(cube_with_sense(Orientation::Reversed));
    let mut volume = 0.0;
    for t in mesh.indices.chunks_exact(3) {
        let a = mesh.positions[t[0] as usize];
        let b = mesh.positions[t[1] as usize];
        let c = mesh.positions[t[2] as usize];
        volume += a.dot(b.cross(c)) / 6.0;
    }
    assert!(
        (volume + 1.0).abs() < 1e-9,
        "reversed shell must invert volume, got {volume}"
    );
}
/// A concave face triangulates by area, not by a fan from vertex zero.
///
/// An L-shape has a reflex corner and its first three vertices are collinear-
/// adjacent, so both a naive fan and a first-two-edge normal fail here while
/// looking fine on a convex quad.
#[test]
fn a_concave_face_keeps_its_true_area() {
    let mut brep = BRep::default();
    // L-shape in the z=0 plane, CCW, area 3 of a 2x2 square.
    // A collinear run at the start (0,0)->(1,0)->(2,0) makes a first-two-edge
    // normal degenerate, which is exactly the case Newell must survive.
    let ring = [
        [0.0, 0.0],
        [1.0, 0.0],
        [2.0, 0.0],
        [2.0, 1.0],
        [1.0, 1.0],
        [1.0, 2.0],
        [0.0, 2.0],
    ];
    let verts: Vec<_> = ring
        .iter()
        .map(|c| {
            brep.add_vertex(Vertex {
                position: Vec3::new(c[0], c[1], 0.0),
            })
        })
        .collect();
    let mut uses = Vec::new();
    for index in 0..verts.len() {
        let edge = brep.add_edge(Edge {
            start: verts[index],
            end: verts[(index + 1) % verts.len()],
            curve: None,
        });
        uses.push(EdgeUse {
            edge,
            orientation: Orientation::Forward,
        });
    }
    let wire = brep.add_loop(Loop { edges: uses });
    let face = brep.add_face(Face {
        surface: None,
        bounds: vec![FaceBound {
            loop_id: wire,
            orientation: Orientation::Forward,
            outer: true,
        }],
        orientation: Orientation::Forward,
    });
    let shell = brep.add_shell(Shell {
        faces: vec![(face, Orientation::Forward)],
        closed: false,
    });
    brep.add_solid(Solid {
        outer: shell,
        voids: Vec::new(),
    });

    let mesh = compile(brep);
    let mut area = 0.0;
    for t in mesh.indices.chunks_exact(3) {
        let a = mesh.positions[t[0] as usize];
        let b = mesh.positions[t[1] as usize];
        let c = mesh.positions[t[2] as usize];
        area += (b - a).cross(c - a).length() / 2.0;
    }
    assert!((area - 3.0).abs() < 1e-9, "L-shape area is 3, got {area}");
    assert_eq!(mesh.indices.len(), 5 * 3, "a 7-gon becomes 5 triangles");
}
/// A reversed bound flips that loop, which flips the face normal.
///
/// IfcFaceBound.Orientation of .F. means traverse the loop backwards. Ignoring
/// it silently mirrors the facet.
#[test]
fn a_reversed_bound_reverses_the_loop() {
    fn triangle(bound_sense: Orientation) -> axiolid_mesh::TriMesh {
        let mut brep = BRep::default();
        let pts = [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]];
        let verts: Vec<_> = pts
            .iter()
            .map(|c| {
                brep.add_vertex(Vertex {
                    position: Vec3::new(c[0], c[1], 0.0),
                })
            })
            .collect();
        let mut uses = Vec::new();
        for index in 0..3 {
            let edge = brep.add_edge(Edge {
                start: verts[index],
                end: verts[(index + 1) % 3],
                curve: None,
            });
            uses.push(EdgeUse {
                edge,
                orientation: Orientation::Forward,
            });
        }
        let wire = brep.add_loop(Loop { edges: uses });
        let face = brep.add_face(Face {
            surface: None,
            bounds: vec![FaceBound {
                loop_id: wire,
                orientation: bound_sense,
                outer: true,
            }],
            orientation: Orientation::Forward,
        });
        let shell = brep.add_shell(Shell {
            faces: vec![(face, Orientation::Forward)],
            closed: false,
        });
        brep.add_solid(Solid {
            outer: shell,
            voids: Vec::new(),
        });
        compile(brep)
    }

    fn normal_z(mesh: &axiolid_mesh::TriMesh) -> f64 {
        let t = &mesh.indices[0..3];
        let a = mesh.positions[t[0] as usize];
        let b = mesh.positions[t[1] as usize];
        let c = mesh.positions[t[2] as usize];
        (b - a).cross(c - a).z
    }

    let forward = normal_z(&triangle(Orientation::Forward));
    let reversed = normal_z(&triangle(Orientation::Reversed));
    assert!(forward > 0.0, "forward bound faces +Z, got {forward}");
    assert!(
        reversed < 0.0,
        "reversed bound must flip the normal, got {reversed}"
    );
}

// --- curved support surfaces ------------------------------------------------

/// A face with a curved support must be refused, not silently faceted.
///
/// Projecting a cylindrical face onto the plane of its boundary yields a mesh
/// that looks valid and is wrong everywhere between the vertices.
#[test]
fn a_curved_face_is_refused_not_flattened() {
    let brep = cube();
    let mut builder = GeometryGraphBuilder::new();
    let cyl = builder
        .push(GeometryNode::Surface(axiolid_surface::Surface::Cylinder(
            axiolid_surface::Cylinder {
                frame: axiolid_core::Frame3 {
                    origin: axiolid_core::Point3::ZERO,
                    x: axiolid_core::Vec3::X,
                    y: axiolid_core::Vec3::Y,
                    z: axiolid_core::Vec3::Z,
                },
                radius: 1.0,
            },
        )))
        .expect("push surface");
    // Attach the curved support to the first face.
    let mut faces: Vec<_> = brep.faces().to_vec();
    faces[0].surface = Some(cyl);
    let mut rebuilt = BRep::default();
    for v in brep.vertices() {
        rebuilt.add_vertex(*v);
    }
    for e in brep.edges() {
        rebuilt.add_edge(e.clone());
    }
    for l in brep.loops() {
        rebuilt.add_loop(l.clone());
    }
    for f in faces {
        rebuilt.add_face(f);
    }
    for s in brep.shells() {
        rebuilt.add_shell(s.clone());
    }
    for s in brep.solids() {
        rebuilt.add_solid(s.clone());
    }
    let root = builder
        .push(GeometryNode::BRep(rebuilt))
        .expect("push brep");
    let graph = builder.finish(vec![root]).expect("finish");
    let compiler = ScalarCompiler::new(BoolmeshBoolean::new());
    let outcome = compiler.compile(
        &graph,
        root,
        &ExecutionOptions::new(axiolid_core::Tolerance::METRE),
    );
    assert!(
        outcome.is_err(),
        "a cylindrical face must be refused, not flattened: {outcome:?}"
    );
}

/// The cube the tessellator actually consumes must audit as a closed
/// manifold. If it does not, either the audit or the fixture is wrong, and
/// every downstream volume claim rests on the answer.
#[test]
fn the_tessellation_cube_is_a_closed_manifold() {
    let health = audit_brep(&cube());
    assert!(
        health.is_closed_manifold(),
        "the cube used for tessellation must be sound: {health:?}"
    );
}
