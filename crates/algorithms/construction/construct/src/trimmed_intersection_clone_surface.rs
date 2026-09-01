use axiolid_contracts::GeomResult;
use axiolid_surface::BSplineSurface;

use crate::trimmed_intersection_builder::allocation_error;

pub(super) fn clone_surface(surface: &BSplineSurface) -> GeomResult<BSplineSurface> {
    Ok(BSplineSurface {
        u_degree: surface.u_degree,
        v_degree: surface.v_degree,
        control_points: clone_rows(&surface.control_points)?,
        u_knots: clone_values(&surface.u_knots)?,
        u_multiplicities: clone_values(&surface.u_multiplicities)?,
        v_knots: clone_values(&surface.v_knots)?,
        v_multiplicities: clone_values(&surface.v_multiplicities)?,
        weights: match &surface.weights {
            Some(weights) => Some(clone_rows(weights)?),
            None => None,
        },
        u_closed: surface.u_closed,
        v_closed: surface.v_closed,
        knot_spec: surface.knot_spec,
        self_intersect: surface.self_intersect,
    })
}

fn clone_rows<T: Copy>(rows: &[Vec<T>]) -> GeomResult<Vec<Vec<T>>> {
    let mut output = Vec::new();
    output
        .try_reserve_exact(rows.len())
        .map_err(|_| allocation_error("trimmed B-rep surface row allocation"))?;
    for row in rows {
        output.push(clone_values(row)?);
    }
    Ok(output)
}

fn clone_values<T: Copy>(values: &[T]) -> GeomResult<Vec<T>> {
    let mut output = Vec::new();
    output
        .try_reserve_exact(values.len())
        .map_err(|_| allocation_error("trimmed B-rep surface value allocation"))?;
    output.extend(values.iter().copied());
    Ok(output)
}
