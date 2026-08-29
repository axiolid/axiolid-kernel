//! Structural validation of a boundary representation.
//!
//! `axiolid-mesh` audits triangle meshes; nothing audited the topology that
//! produces them. A `BRep` could carry dangling references or an
//! unclosed shell and tessellate into silent garbage.
//!
//! `Shell.closed` is a claim the source made, not a fact. This module
//! checks it.

use std::collections::BTreeMap;

use crate::{BRep, Orientation};

/// Structural health of one boundary representation.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct BRepHealth {
    /// References to entities that do not exist.
    pub dangling_references: usize,
    /// Loops whose consecutive edges do not share a vertex.
    pub open_loops: usize,
    /// Faces with no outer bound.
    pub faces_without_outer_bound: usize,
    /// Directed edge uses in the shell that lack an opposing use.
    pub unpaired_edge_uses: usize,
    /// Edges used more than twice in one shell.
    pub overused_edges: usize,
    /// Shells declaring `closed` that are not.
    pub false_closure_claims: usize,
}

impl BRepHealth {
    /// Whether the topology is sound enough to tessellate.
    ///
    /// Closure is deliberately excluded: an open shell is a legitimate
    /// surface model, and refusing it here would reject valid input. What
    /// must never pass is a reference that does not resolve or a loop that
    /// does not close, because both produce silently wrong geometry.
    pub fn is_tessellable(&self) -> bool {
        self.dangling_references == 0 && self.open_loops == 0 && self.faces_without_outer_bound == 0
    }

    /// Whether every shell bounds a volume, so signed-volume reduction and
    /// containment are meaningful.
    pub fn is_closed_manifold(&self) -> bool {
        self.is_tessellable()
            && self.unpaired_edge_uses == 0
            && self.overused_edges == 0
            && self.false_closure_claims == 0
    }
}

/// Audit the structure of a boundary representation.
///
/// Pure topology: no tolerance, no coordinates. Every check is a statement
/// about handles and adjacency, so the result is exact and reproducible.
#[must_use]
pub fn audit_brep<G>(brep: &BRep<G>) -> BRepHealth {
    let mut health = BRepHealth::default();
    let vertices = brep.vertices().len();
    let edges = brep.edges().len();
    let loops = brep.loops().len();
    let faces = brep.faces().len();
    let shells = brep.shells().len();

    for edge in brep.edges() {
        if edge.start.index() >= vertices || edge.end.index() >= vertices {
            health.dangling_references += 1;
        }
    }

    // A loop is closed when consecutive oriented edges meet: the head of one
    // use is the tail of the next. Orientation decides which endpoint is
    // which, so a reversed use that still connects is correct.
    for lp in brep.loops() {
        if lp.edges.is_empty() {
            continue;
        }
        let mut open = false;
        for (k, use_) in lp.edges.iter().enumerate() {
            if use_.edge.index() >= edges {
                health.dangling_references += 1;
                open = true;
                continue;
            }
            let next = &lp.edges[(k + 1) % lp.edges.len()];
            if next.edge.index() >= edges {
                continue;
            }
            let head = endpoints(brep, use_).1;
            let tail = endpoints(brep, next).0;
            if head != tail {
                open = true;
            }
        }
        if open {
            health.open_loops += 1;
        }
    }

    for face in brep.faces() {
        if !face.bounds.iter().any(|b| b.outer) {
            health.faces_without_outer_bound += 1;
        }
        for bound in &face.bounds {
            if bound.loop_id.index() >= loops {
                health.dangling_references += 1;
            }
        }
    }

    // Edge-use pairing per shell. A closed orientable shell uses every edge
    // exactly twice, once in each direction: that is what makes the surface
    // bound a volume. Counting the SIGNED uses catches both a boundary edge
    // (net non-zero) and two faces wound the same way (net non-zero), which
    // a plain "used twice" count would miss.
    for shell in brep.shells() {
        let mut balance: BTreeMap<usize, i32> = BTreeMap::new();
        let mut uses: BTreeMap<usize, usize> = BTreeMap::new();
        for &(face_id, shell_sense) in &shell.faces {
            if face_id.index() >= faces {
                health.dangling_references += 1;
                continue;
            }
            let face = &brep.faces()[face_id.index()];
            let flip = (shell_sense == Orientation::Reversed)
                ^ (face.orientation == Orientation::Reversed);
            for bound in &face.bounds {
                if bound.loop_id.index() >= loops {
                    continue;
                }
                for use_ in &brep.loops()[bound.loop_id.index()].edges {
                    if use_.edge.index() >= edges {
                        continue;
                    }
                    let mut forward = use_.orientation == Orientation::Forward;
                    if flip {
                        forward = !forward;
                    }
                    if bound.orientation == Orientation::Reversed {
                        forward = !forward;
                    }
                    *balance.entry(use_.edge.index()).or_default() += if forward { 1 } else { -1 };
                    *uses.entry(use_.edge.index()).or_default() += 1;
                }
            }
        }
        let unpaired = balance.values().filter(|v| **v != 0).count();
        let overused = uses.values().filter(|c| **c > 2).count();
        health.unpaired_edge_uses += unpaired;
        health.overused_edges += overused;
        if shell.closed && (unpaired > 0 || overused > 0) {
            health.false_closure_claims += 1;
        }
    }
    let _ = shells;
    health
}

/// Ordered endpoints of one oriented edge use.
fn endpoints<G>(brep: &BRep<G>, use_: &crate::EdgeUse<G>) -> (crate::VertexId, crate::VertexId) {
    let edge = &brep.edges()[use_.edge.index()];
    match use_.orientation {
        Orientation::Forward => (edge.start, edge.end),
        _ => (edge.end, edge.start),
    }
}
