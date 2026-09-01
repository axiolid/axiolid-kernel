use axiolid_contracts::{GeomError, GeomResult};
use axiolid_core::Scalar;

pub(crate) fn active_spans(
    knots: &[Scalar],
    multiplicities: &[u32],
    degree: u16,
    count: usize,
) -> GeomResult<Vec<(Scalar, Scalar)>> {
    let invalid = || GeomError::InvalidInput("invalid compact knot vector".to_owned());
    let expected = count
        .checked_add(usize::from(degree))
        .and_then(|value| value.checked_add(1))
        .ok_or_else(invalid)?;
    if knots.is_empty() || knots.len() != multiplicities.len() {
        return Err(invalid());
    }

    let max_multiplicity = u32::from(degree) + 1;
    let mut total = 0_usize;
    for &multiplicity in multiplicities {
        if multiplicity == 0 || multiplicity > max_multiplicity {
            return Err(invalid());
        }
        total = total
            .checked_add(multiplicity as usize)
            .ok_or_else(invalid)?;
        if total > expected {
            return Err(invalid());
        }
    }
    if total != expected {
        return Err(invalid());
    }

    let mut expanded = Vec::with_capacity(expected);
    for (&knot, &multiplicity) in knots.iter().zip(multiplicities) {
        expanded.extend(core::iter::repeat_n(knot, multiplicity as usize));
    }
    let lo = usize::from(degree);
    let mut spans = Vec::new();
    for pair in expanded[lo..=count].windows(2) {
        if pair[1] > pair[0] {
            spans.push((pair[0], pair[1]));
        }
    }
    if spans.is_empty() {
        Err(GeomError::Degenerate("spline domain is empty".to_owned()))
    } else {
        Ok(spans)
    }
}

pub(crate) fn reverse_axis(
    knots: &[Scalar],
    multiplicities: &[u32],
) -> GeomResult<(Vec<Scalar>, Vec<u32>)> {
    let sum = knots
        .first()
        .copied()
        .zip(knots.last().copied())
        .map(|(first, last)| first + last)
        .ok_or_else(|| GeomError::InvalidInput("B-spline axis has no knots".to_owned()))?;
    let knots: Vec<_> = knots.iter().rev().map(|&knot| sum - knot).collect();
    if knots.iter().any(|knot| !knot.is_finite()) {
        return Err(GeomError::InvalidInput(
            "reversed B-spline axis knots must be finite".to_owned(),
        ));
    }
    if knots.windows(2).any(|pair| pair[1] <= pair[0]) {
        return Err(GeomError::InvalidInput(
            "reversed B-spline axis knots are not strictly increasing".to_owned(),
        ));
    }

    Ok((knots, multiplicities.iter().rev().copied().collect()))
}
