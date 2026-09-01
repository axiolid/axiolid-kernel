use axiolid_core::{Point2, Scalar};
use axiolid_nurbs::{
    ParameterInterval, SurfaceSurfaceTraceEndpoint3, TransverseSurfaceSurfaceTrace3,
};
use axiolid_surface::BSplineSurface;

use crate::trimmed_intersection_types::SurfacePairMember;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum BoundarySide {
    VStart,
    UEnd,
    VEnd,
    UStart,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct Domain2 {
    pub u_start: Scalar,
    pub u_end: Scalar,
    pub v_start: Scalar,
    pub v_end: Scalar,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct Endpoint2 {
    pub uv: Point2,
    pub side: Option<BoundarySide>,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct SplitClassification {
    pub member: SurfacePairMember,
    pub owner_domain: Domain2,
    pub embedded_domain: Domain2,
    pub owner_start: Endpoint2,
    pub owner_end: Endpoint2,
    pub embedded_start: Endpoint2,
    pub embedded_end: Endpoint2,
}

pub(super) fn classify(
    first: &BSplineSurface,
    second: &BSplineSurface,
    trace: &TransverseSurfaceSurfaceTrace3,
) -> Option<SplitClassification> {
    let first_domain = domain(first)?;
    let second_domain = domain(second)?;
    let first_start = endpoint(&trace.start, SurfacePairMember::First, first_domain)?;
    let first_end = endpoint(&trace.end, SurfacePairMember::First, first_domain)?;
    let second_start = endpoint(&trace.start, SurfacePairMember::Second, second_domain)?;
    let second_end = endpoint(&trace.end, SurfacePairMember::Second, second_domain)?;

    let first_owns = owns_chord(first_start, first_end, first_domain);
    let second_owns = owns_chord(second_start, second_end, second_domain);
    let first_embeds = interior(first_start, first_domain) && interior(first_end, first_domain);
    let second_embeds =
        interior(second_start, second_domain) && interior(second_end, second_domain);

    match (first_owns, second_owns, first_embeds, second_embeds) {
        (true, false, false, true) => Some(SplitClassification {
            member: SurfacePairMember::First,
            owner_domain: first_domain,
            embedded_domain: second_domain,
            owner_start: first_start,
            owner_end: first_end,
            embedded_start: second_start,
            embedded_end: second_end,
        }),
        (false, true, true, false) => Some(SplitClassification {
            member: SurfacePairMember::Second,
            owner_domain: second_domain,
            embedded_domain: first_domain,
            owner_start: second_start,
            owner_end: second_end,
            embedded_start: first_start,
            embedded_end: first_end,
        }),
        _ => None,
    }
}

fn owns_chord(start: Endpoint2, end: Endpoint2, domain: Domain2) -> bool {
    matches!((start.side, end.side), (Some(left), Some(right)) if left != right)
        && side_interior(start, domain)
        && side_interior(end, domain)
        && start.uv != end.uv
}

fn side_interior(endpoint: Endpoint2, domain: Domain2) -> bool {
    match endpoint.side {
        Some(BoundarySide::VStart | BoundarySide::VEnd) => {
            endpoint.uv.x > domain.u_start && endpoint.uv.x < domain.u_end
        }
        Some(BoundarySide::UStart | BoundarySide::UEnd) => {
            endpoint.uv.y > domain.v_start && endpoint.uv.y < domain.v_end
        }
        None => false,
    }
}

fn interior(endpoint: Endpoint2, domain: Domain2) -> bool {
    endpoint.side.is_none()
        && endpoint.uv.x > domain.u_start
        && endpoint.uv.x < domain.u_end
        && endpoint.uv.y > domain.v_start
        && endpoint.uv.y < domain.v_end
}

fn domain(surface: &BSplineSurface) -> Option<Domain2> {
    let u_start = *surface.u_knots.first()?;
    let u_end = *surface.u_knots.last()?;
    let v_start = *surface.v_knots.first()?;
    let v_end = *surface.v_knots.last()?;
    let values = [u_start, u_end, v_start, v_end];
    if values.iter().all(|value| value.is_finite()) && u_start < u_end && v_start < v_end {
        Some(Domain2 {
            u_start,
            u_end,
            v_start,
            v_end,
        })
    } else {
        None
    }
}

fn endpoint(
    endpoint: &SurfaceSurfaceTraceEndpoint3,
    member: SurfacePairMember,
    domain: Domain2,
) -> Option<Endpoint2> {
    let parameters = endpoint.parameters;
    let (u, v) = match member {
        SurfacePairMember::First => (parameters.first_u, parameters.first_v),
        SurfacePairMember::Second => (parameters.second_u, parameters.second_v),
    };
    let uv = Point2::new(midpoint(u)?, midpoint(v)?);
    if uv.x < domain.u_start || uv.x > domain.u_end || uv.y < domain.v_start || uv.y > domain.v_end
    {
        return None;
    }
    let mut side = None;
    for candidate in [
        fixed_side(v, domain.v_start, BoundarySide::VStart),
        fixed_side(u, domain.u_end, BoundarySide::UEnd),
        fixed_side(v, domain.v_end, BoundarySide::VEnd),
        fixed_side(u, domain.u_start, BoundarySide::UStart),
    ]
    .into_iter()
    .flatten()
    {
        if side.replace(candidate).is_some() {
            return None;
        }
    }
    Some(Endpoint2 { uv, side })
}

fn fixed_side(
    interval: ParameterInterval,
    boundary: Scalar,
    side: BoundarySide,
) -> Option<BoundarySide> {
    (interval.start == boundary && interval.end == boundary).then_some(side)
}

fn midpoint(interval: ParameterInterval) -> Option<Scalar> {
    if !interval.start.is_finite() || !interval.end.is_finite() || interval.start > interval.end {
        return None;
    }
    let value = interval.start * 0.5 + interval.end * 0.5;
    value.is_finite().then_some(value)
}

pub(super) fn boundary_rank(side: BoundarySide, uv: Point2, domain: Domain2) -> Scalar {
    match side {
        BoundarySide::VStart => (uv.x - domain.u_start) / (domain.u_end - domain.u_start),
        BoundarySide::UEnd => 1.0 + (uv.y - domain.v_start) / (domain.v_end - domain.v_start),
        BoundarySide::VEnd => 2.0 + (domain.u_end - uv.x) / (domain.u_end - domain.u_start),
        BoundarySide::UStart => 3.0 + (domain.v_end - uv.y) / (domain.v_end - domain.v_start),
    }
}
