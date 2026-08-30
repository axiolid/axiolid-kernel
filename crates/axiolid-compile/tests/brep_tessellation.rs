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
                pcurve: None,
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
            pcurve: None,
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
            pcurve: None,
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
                pcurve: None,
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

/// A curved face WITH a pcurve is sampled on its surface, not flattened.
///
/// The trim states the face boundary in `(u, v)`, so the sampler knows which
/// part of the cylinder to emit. Radius is the check that it really sampled
/// the surface: every vertex must sit `r` from the axis, which a planar
/// projection through the boundary could not achieve.
#[test]
fn a_curved_face_with_a_pcurve_is_sampled_on_its_surface() {
    let radius = 2.0;
    let mut builder = GeometryGraphBuilder::new();
    let surface = builder
        .push(GeometryNode::Surface(axiolid_surface::Surface::Cylinder(
            axiolid_surface::Cylinder {
                frame: axiolid_core::Frame3 {
                    origin: axiolid_core::Point3::ZERO,
                    x: axiolid_core::Vec3::X,
                    y: axiolid_core::Vec3::Y,
                    z: axiolid_core::Vec3::Z,
                },
                radius,
            },
        )))
        .expect("surface");
    // A rectangle in parameter space: a quarter turn, two metres tall.
    let quarter = std::f64::consts::FRAC_PI_2;
    let trim = builder
        .push(GeometryNode::Curve2(axiolid_curve::Curve2::Polyline(
            axiolid_curve::Polyline2 {
                points: vec![
                    axiolid_core::Point2::new(0.0, 0.0),
                    axiolid_core::Point2::new(quarter, 0.0),
                    axiolid_core::Point2::new(quarter, 2.0),
                    axiolid_core::Point2::new(0.0, 2.0),
                ],
                closed: true,
            },
        )))
        .expect("trim");

    let mut brep: BRep<axiolid_model::NodeId> = BRep::default();
    let v: Vec<_> = (0..4)
        .map(|i| {
            let a = quarter * f64::from(i % 2);
            brep.add_vertex(Vertex {
                position: axiolid_core::Point3::new(radius * a.cos(), radius * a.sin(), 0.0),
            })
        })
        .collect();
    let e: Vec<_> = (0..4)
        .map(|i| {
            brep.add_edge(Edge {
                start: v[i],
                end: v[(i + 1) % 4],
                curve: None,
            })
        })
        .collect();
    let wire = brep.add_loop(Loop {
        edges: e
            .iter()
            .map(|&edge| EdgeUse {
                edge,
                orientation: Orientation::Forward,
                pcurve: Some(trim),
            })
            .collect(),
    });
    let face = brep.add_face(Face {
        surface: Some(surface),
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

    let root = builder.push(GeometryNode::BRep(brep)).expect("brep");
    let graph = builder.finish(vec![root]).expect("finish");
    let compiler = ScalarCompiler::new(BoolmeshBoolean::new());
    let mesh = compiler
        .compile(
            &graph,
            root,
            &ExecutionOptions::new(axiolid_core::Tolerance::MILLIMETRE),
        )
        .expect("a curved face with a trim must tessellate");

    assert!(
        mesh.positions.len() > 8,
        "a sampled cylinder needs more than the boundary vertices, got {}",
        mesh.positions.len()
    );
    for p in &mesh.positions {
        let r = (p.x * p.x + p.y * p.y).sqrt();
        assert!(
            (r - radius).abs() < 1e-9,
            "vertex {p:?} is {r} from the axis, not {radius}: the face was \
             flattened rather than sampled"
        );
    }
}

/// Broken topology is refused before any triangle is emitted.
///
/// An open loop cannot bound a face, but the planar path would happily
/// project its points and return a plausible-looking mesh. The audit runs
/// first precisely so that never reaches geometry.
#[test]
fn unsound_topology_is_refused_before_tessellation() {
    let mut brep: BRep<axiolid_model::NodeId> = BRep::default();
    let v: Vec<_> = [
        axiolid_core::Point3::new(0.0, 0.0, 0.0),
        axiolid_core::Point3::new(1.0, 0.0, 0.0),
        axiolid_core::Point3::new(1.0, 1.0, 0.0),
        axiolid_core::Point3::new(0.0, 1.0, 0.0),
    ]
    .into_iter()
    .map(|position| brep.add_vertex(Vertex { position }))
    .collect();
    // Three disjoint edges: the loop cannot close.
    let e0 = brep.add_edge(Edge {
        start: v[0],
        end: v[1],
        curve: None,
    });
    let e1 = brep.add_edge(Edge {
        start: v[2],
        end: v[3],
        curve: None,
    });
    let wire = brep.add_loop(Loop {
        edges: vec![
            EdgeUse {
                edge: e0,
                orientation: Orientation::Forward,
                pcurve: None,
            },
            EdgeUse {
                edge: e1,
                orientation: Orientation::Forward,
                pcurve: None,
            },
        ],
    });
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

    let mut builder = GeometryGraphBuilder::new();
    let root = builder.push(GeometryNode::BRep(brep)).expect("push");
    let graph = builder.finish(vec![root]).expect("finish");
    let compiler = ScalarCompiler::new(BoolmeshBoolean::new());
    let result = compiler.compile(
        &graph,
        root,
        &ExecutionOptions::new(axiolid_core::Tolerance::METRE),
    );
    assert!(
        result.is_err(),
        "an open loop must be refused, not tessellated into a plausible mesh"
    );
}

/// Two curved faces meeting at a seam must share its vertices.
///
/// Each face owns its own pcurve. Evaluating both independently lands on the
/// same 3D curve at different parameters, so the seam looks closed and is
/// not. Sampling each edge once and reusing the indices is what closes it.
#[test]
fn two_curved_faces_share_their_seam_vertices() {
    use axiolid_core::{Point2, Point3, Scalar, Vec3};
    use axiolid_curve::{Curve2, Polyline2};
    use axiolid_surface::{Cylinder, Surface};

    let mut builder = GeometryGraphBuilder::new();
    let radius = 2.0;
    let frame = axiolid_core::Frame3 {
        origin: Point3::ZERO,
        x: Vec3::X,
        y: Vec3::Y,
        z: Vec3::Z,
    };
    let surface = builder
        .push(GeometryNode::Surface(Surface::Cylinder(Cylinder {
            frame,
            radius,
        })))
        .expect("surface");

    // Half-cylinder split at u = PI. Both faces use the same seam EDGE,
    // each with its own pcurve node.
    let pi = core::f64::consts::PI;
    let line = |b: &mut GeometryGraphBuilder, a: Point2, c: Point2| {
        b.push(GeometryNode::Curve2(Curve2::Polyline(Polyline2 {
            points: vec![a, c],
            closed: false,
        })))
        .expect("pcurve")
    };

    let mut brep: BRep<axiolid_model::NodeId> = BRep::default();
    let p = |u: Scalar, v: Scalar| Point3::new(radius * u.cos(), radius * u.sin(), v);
    let v00 = brep.add_vertex(Vertex {
        position: p(0.0, 0.0),
    });
    let v01 = brep.add_vertex(Vertex {
        position: p(0.0, 3.0),
    });
    let v10 = brep.add_vertex(Vertex {
        position: p(pi, 0.0),
    });
    let v11 = brep.add_vertex(Vertex {
        position: p(pi, 3.0),
    });

    // The shared seam edge, plus the three others bounding face A.
    let seam = brep.add_edge(Edge {
        start: v10,
        end: v11,
        curve: None,
    });
    let bottom = brep.add_edge(Edge {
        start: v00,
        end: v10,
        curve: None,
    });
    let top = brep.add_edge(Edge {
        start: v01,
        end: v11,
        curve: None,
    });
    let start = brep.add_edge(Edge {
        start: v00,
        end: v01,
        curve: None,
    });

    // Face A trims: u from 0 to PI.
    let a_bottom = line(&mut builder, Point2::new(0.0, 0.0), Point2::new(pi, 0.0));
    let a_seam = line(&mut builder, Point2::new(pi, 0.0), Point2::new(pi, 3.0));
    let a_top = line(&mut builder, Point2::new(pi, 3.0), Point2::new(0.0, 3.0));
    let a_start = line(&mut builder, Point2::new(0.0, 3.0), Point2::new(0.0, 0.0));

    let loop_a = brep.add_loop(Loop {
        edges: vec![
            EdgeUse {
                edge: bottom,
                orientation: Orientation::Forward,
                pcurve: Some(a_bottom),
            },
            EdgeUse {
                edge: seam,
                orientation: Orientation::Forward,
                pcurve: Some(a_seam),
            },
            EdgeUse {
                edge: top,
                orientation: Orientation::Reversed,
                pcurve: Some(a_top),
            },
            EdgeUse {
                edge: start,
                orientation: Orientation::Reversed,
                pcurve: Some(a_start),
            },
        ],
    });
    let face_a = brep.add_face(Face {
        surface: Some(surface),
        bounds: vec![FaceBound {
            loop_id: loop_a,
            orientation: Orientation::Forward,
            outer: true,
        }],
        orientation: Orientation::Forward,
    });

    // Face B covers u from PI to TAU and walks the SAME seam edge backwards.
    let tau = core::f64::consts::TAU;
    let v20 = brep.add_vertex(Vertex {
        position: p(tau, 0.0),
    });
    let v21 = brep.add_vertex(Vertex {
        position: p(tau, 3.0),
    });
    let b_bottom_e = brep.add_edge(Edge {
        start: v10,
        end: v20,
        curve: None,
    });
    let b_top_e = brep.add_edge(Edge {
        start: v11,
        end: v21,
        curve: None,
    });
    let b_end_e = brep.add_edge(Edge {
        start: v20,
        end: v21,
        curve: None,
    });

    let b_seam = line(&mut builder, Point2::new(pi, 3.0), Point2::new(pi, 0.0));
    let b_bottom = line(&mut builder, Point2::new(pi, 0.0), Point2::new(tau, 0.0));
    let b_end = line(&mut builder, Point2::new(tau, 0.0), Point2::new(tau, 3.0));
    let b_top = line(&mut builder, Point2::new(tau, 3.0), Point2::new(pi, 3.0));

    let loop_b = brep.add_loop(Loop {
        edges: vec![
            EdgeUse {
                edge: seam,
                orientation: Orientation::Reversed,
                pcurve: Some(b_seam),
            },
            EdgeUse {
                edge: b_bottom_e,
                orientation: Orientation::Forward,
                pcurve: Some(b_bottom),
            },
            EdgeUse {
                edge: b_end_e,
                orientation: Orientation::Forward,
                pcurve: Some(b_end),
            },
            EdgeUse {
                edge: b_top_e,
                orientation: Orientation::Reversed,
                pcurve: Some(b_top),
            },
        ],
    });
    let face_b = brep.add_face(Face {
        surface: Some(surface),
        bounds: vec![FaceBound {
            loop_id: loop_b,
            orientation: Orientation::Forward,
            outer: true,
        }],
        orientation: Orientation::Forward,
    });

    let shell = brep.add_shell(Shell {
        faces: vec![
            (face_a, Orientation::Forward),
            (face_b, Orientation::Forward),
        ],
        closed: false,
    });
    let _ = brep.add_solid(Solid {
        outer: shell,
        voids: Vec::new(),
    });

    let root = builder.push(GeometryNode::BRep(brep)).expect("push");
    let graph = builder.finish(vec![root]).expect("finish");
    let compiler = ScalarCompiler::new(BoolmeshBoolean::new());
    let mesh = compiler
        .compile(
            &graph,
            root,
            &ExecutionOptions::new(axiolid_core::Tolerance::MILLIMETRE),
        )
        .expect("two curved faces compile");

    // Every vertex is on the cylinder.
    for q in &mesh.positions {
        let r = (q.x * q.x + q.y * q.y).sqrt();
        assert!((r - radius).abs() < 1e-9, "vertex off the cylinder: {r}");
    }

    // The seam is shared: the interior edge along u = PI must be used by
    // exactly two triangles, one from each face. Duplicated seam vertices
    // would make it two separate boundary edges instead.
    let mut edges: std::collections::HashMap<(u32, u32), i32> = std::collections::HashMap::new();
    for t in mesh.indices.chunks_exact(3) {
        for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
            let key = if a < b { (a, b) } else { (b, a) };
            *edges.entry(key).or_default() += if a < b { 1 } else { -1 };
        }
    }
    let interior = edges.values().filter(|v| **v == 0).count();
    assert!(
        interior > 0,
        "no interior edge is shared: the seam was duplicated"
    );
}

/// A full cylinder closes across its periodic seam.
///
/// The seam edge appears TWICE in one loop: once at u = 0 and once at
/// u = TAU. Both uses name the same EdgeId, so both must resolve to the
/// same 3D vertices or the tube is split down its length.
#[test]
fn a_periodic_seam_closes_the_tube() {
    use axiolid_core::{Point2, Vec3};
    use axiolid_curve::{Curve2, Polyline2};
    use axiolid_surface::{Cylinder, Surface};
    use core::f64::consts::TAU;

    let radius = 2.0;
    let height = 3.0;
    let mut builder = GeometryGraphBuilder::new();
    let surface = builder
        .push(GeometryNode::Surface(Surface::Cylinder(Cylinder {
            frame: axiolid_core::Frame3 {
                origin: axiolid_core::Point3::ZERO,
                x: Vec3::X,
                y: Vec3::Y,
                z: Vec3::Z,
            },
            radius,
        })))
        .expect("surface");
    // Trims in (u, v). The loop walks: bottom rim u 0->TAU, seam up at
    // u = TAU, top rim back TAU->0, seam down at u = 0. The two seam uses
    // name ONE edge, so the tube closes only if both resolve alike.
    let bottom = builder
        .push(GeometryNode::Curve2(Curve2::Polyline(Polyline2 {
            points: vec![Point2::new(0.0, 0.0), Point2::new(TAU, 0.0)],
            closed: false,
        })))
        .expect("bottom");
    let top = builder
        .push(GeometryNode::Curve2(Curve2::Polyline(Polyline2 {
            points: vec![Point2::new(TAU, height), Point2::new(0.0, height)],
            closed: false,
        })))
        .expect("top");
    // The seam's two uses have DIFFERENT pcurves -- u = TAU going up and
    // u = 0 coming down -- but name the same topological edge.
    let seam_up = builder
        .push(GeometryNode::Curve2(Curve2::Polyline(Polyline2 {
            points: vec![Point2::new(TAU, 0.0), Point2::new(TAU, height)],
            closed: false,
        })))
        .expect("seam up");
    let seam_down = builder
        .push(GeometryNode::Curve2(Curve2::Polyline(Polyline2 {
            points: vec![Point2::new(0.0, height), Point2::new(0.0, 0.0)],
            closed: false,
        })))
        .expect("seam down");
    let mut brep: BRep<axiolid_model::NodeId> = BRep::default();
    let p = |u: f64, v: f64| axiolid_core::Point3::new(radius * u.cos(), radius * u.sin(), v);
    let v00 = brep.add_vertex(Vertex {
        position: p(0.0, 0.0),
    });
    let v01 = brep.add_vertex(Vertex {
        position: p(0.0, height),
    });
    // Rims start and end at the seam vertices: a periodic loop has no
    // other corners.
    let e_bottom = brep.add_edge(Edge {
        start: v00,
        end: v00,
        curve: None,
    });
    let e_seam = brep.add_edge(Edge {
        start: v00,
        end: v01,
        curve: None,
    });
    let e_top = brep.add_edge(Edge {
        start: v01,
        end: v01,
        curve: None,
    });
    let wire = brep.add_loop(Loop {
        edges: vec![
            EdgeUse {
                edge: e_bottom,
                orientation: Orientation::Forward,
                pcurve: Some(bottom),
            },
            EdgeUse {
                edge: e_seam,
                orientation: Orientation::Forward,
                pcurve: Some(seam_up),
            },
            EdgeUse {
                edge: e_top,
                orientation: Orientation::Forward,
                pcurve: Some(top),
            },
            EdgeUse {
                edge: e_seam,
                orientation: Orientation::Reversed,
                pcurve: Some(seam_down),
            },
        ],
    });
    let face = brep.add_face(Face {
        surface: Some(surface),
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

    let root = builder.push(GeometryNode::BRep(brep)).expect("brep");
    let graph = builder.finish(vec![root]).expect("finish");
    let mesh = ScalarCompiler::new(BoolmeshBoolean::new())
        .compile(
            &graph,
            root,
            &ExecutionOptions::new(axiolid_core::Tolerance::MILLIMETRE),
        )
        .expect("periodic cylinder tessellates");

    // Every vertex sits on the cylinder.
    for p in &mesh.positions {
        let r = (p.x * p.x + p.y * p.y).sqrt();
        assert!(
            (r - radius).abs() < 1e-6,
            "vertex off the cylinder: r = {r}"
        );
    }
    // The seam must be interior: no vertical run of boundary edges at u = 0.
    let mut edges: std::collections::HashMap<(u32, u32), i32> = Default::default();
    for t in mesh.indices.chunks_exact(3) {
        for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
            let key = if a < b { (a, b) } else { (b, a) };
            *edges.entry(key).or_default() += if a < b { 1 } else { -1 };
        }
    }
    let seam_open = edges
        .iter()
        .filter(|(_, &bal)| bal != 0)
        .filter(|((a, b), _)| {
            let (pa, pb) = (mesh.positions[*a as usize], mesh.positions[*b as usize]);
            (pa.z - pb.z).abs() > 1e-9 && pa.y.abs() < 1e-6 && pb.y.abs() < 1e-6 && pa.x > 0.0
        })
        .count();
    assert_eq!(
        seam_open, 0,
        "the periodic seam is split: {seam_open} open edges at u = 0"
    );
}
