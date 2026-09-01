//! Scalar reference tessellation of surfaces (ADR 0012).
//!
//! # What this closes
//!
//! `axiolid-tessellation-contract` declares a `Tessellator` trait that nothing
//! implemented, so no curved face could ever become triangles. A B-rep with a
//! cylindrical or spline face was unreachable, which is most curved
//! geometry in a real building model.
//!
//! # Method
//!
//! Uniform parameter sampling with the step chosen from a measured sagitta,
//! not a guessed segment count. For each parameter direction the maximum
//! deviation of the chord from the surface is probed at the midpoint, and the
//! step is halved until the deviation is within the chord budget or a caller
//! budget is exhausted. That is the same contract `flatten2` uses for curves,
//! so a cylinder tessellated here and its silhouette circle flattened there
//! agree on what a tolerance means.
//!
//! Adaptive *quad-tree* refinement would use fewer triangles on surfaces with
//! localised curvature. It is deliberately not done here: this is the
//! reference implementation, and a uniform grid is the version whose output a
//! human can predict and check by hand. An optimised backend may refine
//! locally and be validated against this one.

use axiolid_contracts::{GeomError, GeomResult};
use axiolid_core::{Point3, Scalar};
use axiolid_mesh::TriMesh;
use axiolid_surface::Surface;

use crate::surface::{evaluate, Patch};

/// Sampling budget for one surface.
///
/// Explicit rather than defaulted: the acceptable triangle count depends on
/// what the mesh is for. A clash test wants a coarse conservative hull; a
/// quantity takeoff wants convergence. The kernel does not know which.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TessellationBudget {
    /// Maximum chord deviation from the true surface.
    pub chord_tolerance: Scalar,
    /// Hard cap on samples per parameter direction.
    pub max_samples_per_direction: usize,
}

impl TessellationBudget {
    /// Construct a validated budget.
    pub fn new(chord_tolerance: Scalar, max_samples_per_direction: usize) -> GeomResult<Self> {
        if !(chord_tolerance.is_finite() && chord_tolerance > 0.0) {
            return Err(GeomError::InvalidInput(format!(
                "chord tolerance must be positive and finite, got {chord_tolerance}"
            )));
        }
        if max_samples_per_direction < 2 {
            return Err(GeomError::InvalidInput(format!(
                "need at least 2 samples per direction, got {max_samples_per_direction}"
            )));
        }
        Ok(Self {
            chord_tolerance,
            max_samples_per_direction,
        })
    }
}

/// A tessellated patch plus the evidence needed to judge it.
#[derive(Debug, Clone)]
pub struct TessellationOutcome {
    /// The triangulated patch.
    pub mesh: TriMesh,
    /// Samples used along u.
    pub u_samples: usize,
    /// Samples used along v.
    pub v_samples: usize,
    /// Largest measured chord deviation, or `None` when the surface is flat
    /// in that direction and no deviation was observable.
    pub max_sagitta: Option<Scalar>,
    /// Whether the budget was exhausted before the tolerance was met. A
    /// caller measuring quantities must treat this as a failed measurement,
    /// not a coarse one.
    pub budget_exhausted: bool,
}

/// Tessellate one bounded surface patch.
///
/// The patch is required: an elementary surface is infinite, and there is no
/// defensible default bound for one. Returning a guessed extent would be a
/// silent modelling decision.
pub fn tessellate_patch(
    surface: &Surface,
    patch: Patch,
    budget: TessellationBudget,
) -> GeomResult<TessellationOutcome> {
    let (nu, su) = resolve_samples(surface, patch, budget, Direction::U)?;
    let (nv, sv) = resolve_samples(surface, patch, budget, Direction::V)?;
    let exhausted =
        nu >= budget.max_samples_per_direction || nv >= budget.max_samples_per_direction;

    let mut positions = Vec::with_capacity(nu * nv);
    for i in 0..nu {
        let u = lerp(patch.u_start, patch.u_end, i, nu);
        for j in 0..nv {
            let v = lerp(patch.v_start, patch.v_end, j, nv);
            positions.push(evaluate(surface, u, v)?);
        }
    }

    // Grid connectivity. Each cell splits along the diagonal that keeps the
    // two triangles closest in area, which is what keeps a highly anisotropic
    // patch (a long thin cylinder band) from producing slivers.
    let mut indices = Vec::with_capacity((nu - 1) * (nv - 1) * 6);
    for i in 0..nu - 1 {
        for j in 0..nv - 1 {
            let a = (i * nv + j) as u32;
            let b = (i * nv + j + 1) as u32;
            let c = ((i + 1) * nv + j) as u32;
            let d = ((i + 1) * nv + j + 1) as u32;
            if shorter_diagonal_is_ad(&positions, a, b, c, d) {
                indices.extend_from_slice(&[a, c, d]);
                indices.extend_from_slice(&[a, d, b]);
            } else {
                indices.extend_from_slice(&[a, c, b]);
                indices.extend_from_slice(&[b, c, d]);
            }
        }
    }

    let max_sagitta = match (su, sv) {
        (Some(a), Some(b)) => Some(a.max(b)),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    };

    Ok(TessellationOutcome {
        mesh: TriMesh::new(positions, indices),
        u_samples: nu,
        v_samples: nv,
        max_sagitta,
        budget_exhausted: exhausted,
    })
}

/// Which parameter direction a sample count applies to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Direction {
    /// First surface parameter.
    U,
    /// Second surface parameter.
    V,
}

/// Uniform sample position `i` of `n` across `[a, b]`.
///
/// Computed from the endpoints rather than by accumulating a step so the last
/// sample is exactly `b`. An accumulated step leaves a gap that reopens a
/// closed surface into a sliver.
fn lerp(a: Scalar, b: Scalar, i: usize, n: usize) -> Scalar {
    if n <= 1 {
        return a;
    }
    let t = i as Scalar / (n - 1) as Scalar;
    a + (b - a) * t
}

/// Whether the `a-d` diagonal is shorter than `b-c` for one grid cell.
///
/// Splitting along the shorter diagonal keeps the two triangles closer in
/// shape. On an anisotropic patch the wrong choice produces slivers, which
/// `audit_mesh` then reports as degenerate and every downstream measure
/// refuses.
fn shorter_diagonal_is_ad(positions: &[Point3], a: u32, b: u32, c: u32, d: u32) -> bool {
    let p = |i: u32| positions[i as usize];
    (p(a) - p(d)).length_squared() <= (p(b) - p(c)).length_squared()
}

/// Choose a sample count for one direction by measuring, not guessing.
///
/// Doubles the interval count until the midpoint sagitta of every span is
/// within the chord budget. Returns the count and the largest measured
/// deviation; `None` means no deviation was observable, which is the honest
/// answer for a direction in which the surface is straight.
fn resolve_samples(
    surface: &Surface,
    patch: Patch,
    budget: TessellationBudget,
    direction: Direction,
) -> GeomResult<(usize, Option<Scalar>)> {
    let mut n = 2usize;
    loop {
        let worst = worst_sagitta(surface, patch, direction, n)?;
        match worst {
            Some(s) if s > budget.chord_tolerance => {}
            // Within tolerance, or flat: this count stands.
            _ => return Ok((n, worst)),
        }
        if n >= budget.max_samples_per_direction {
            return Ok((n, worst));
        }
        // Grow the interval count, never exceeding the caller's cap.
        n = (2 * (n - 1) + 1).min(budget.max_samples_per_direction);
    }
}

/// Largest chord-to-surface deviation over all spans in one direction.
///
/// The deviation is probed at each span midpoint against the chord joining the
/// span ends, sampled at a fixed set of positions in the other parameter so an
/// error that only appears away from the patch border is still seen.
fn worst_sagitta(
    surface: &Surface,
    patch: Patch,
    direction: Direction,
    n: usize,
) -> GeomResult<Option<Scalar>> {
    const CROSS_PROBES: usize = 3;
    let mut worst: Option<Scalar> = None;
    for span in 0..n.saturating_sub(1) {
        for k in 0..CROSS_PROBES {
            let (a, b, mid, other) = span_probe(patch, direction, span, n, k, CROSS_PROBES);
            let pa = eval_at(surface, direction, a, other)?;
            let pb = eval_at(surface, direction, b, other)?;
            let pm = eval_at(surface, direction, mid, other)?;
            let deviation = point_to_segment(pm, pa, pb);
            worst = Some(worst.map_or(deviation, |w: Scalar| w.max(deviation)));
        }
    }
    Ok(worst)
}

/// Parameters for one sagitta probe: span ends, span midpoint, cross position.
fn span_probe(
    patch: Patch,
    direction: Direction,
    span: usize,
    n: usize,
    k: usize,
    probes: usize,
) -> (Scalar, Scalar, Scalar, Scalar) {
    let (start, end, cross_start, cross_end) = match direction {
        Direction::U => (patch.u_start, patch.u_end, patch.v_start, patch.v_end),
        Direction::V => (patch.v_start, patch.v_end, patch.u_start, patch.u_end),
    };
    let a = lerp(start, end, span, n);
    let b = lerp(start, end, span + 1, n);
    let other = lerp(cross_start, cross_end, k, probes);
    (a, b, 0.5 * (a + b), other)
}

/// Evaluate with the two parameters ordered by `direction`.
fn eval_at(
    surface: &Surface,
    direction: Direction,
    along: Scalar,
    other: Scalar,
) -> GeomResult<Point3> {
    match direction {
        Direction::U => evaluate(surface, along, other),
        Direction::V => evaluate(surface, other, along),
    }
}

/// Distance from `m` to segment `a-b`.
///
/// Projects onto the segment and clamps, so a midpoint that falls beyond an
/// end reports the true distance rather than the distance to the infinite
/// line, which would understate the deviation.
fn point_to_segment(m: Point3, a: Point3, b: Point3) -> Scalar {
    let ab = b - a;
    let len2 = ab.length_squared();
    if len2 <= 0.0 {
        return (m - a).length();
    }
    // Clamped because this measures distance to a finite SEGMENT, not to its
    // infinite line. `tessellate_patch` only ever passes a chord midpoint,
    // whose foot is always interior, so the clamp is unreachable from there
    // and no mutation probe can kill it. It is kept for callers that pass an
    // arbitrary point, where dropping it would silently under-report.
    let t = ((m - a).dot(ab) / len2).clamp(0.0, 1.0);
    (m - (a + ab * t)).length()
}
