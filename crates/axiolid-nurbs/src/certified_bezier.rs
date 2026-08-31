//! Outward-rounded homogeneous Bézier enclosures for certification algorithms.

use axiolid_core::Scalar;
use axiolid_kernel::{GeomError, GeomResult};

#[derive(Debug, Clone, Copy)]
pub(crate) struct Interval {
    lo: Scalar,
    hi: Scalar,
}

impl Interval {
    pub(crate) fn exact(value: Scalar) -> GeomResult<Self> {
        if !value.is_finite() {
            return Err(GeomError::InvalidInput(
                "certified projection inputs must be finite".to_owned(),
            ));
        }
        Ok(Self {
            lo: value,
            hi: value,
        })
    }

    pub(crate) fn product(left: Scalar, right: Scalar) -> GeomResult<Self> {
        let value = left * right;
        if !value.is_finite() {
            return Err(GeomError::InvalidInput(
                "homogeneous control coordinate overflows".to_owned(),
            ));
        }
        Ok(Self {
            lo: next_down(value),
            hi: next_up(value),
        })
    }

    fn add(self, other: Self) -> GeomResult<Self> {
        let lo = self.lo + other.lo;
        let hi = self.hi + other.hi;
        if !lo.is_finite() || !hi.is_finite() {
            return Err(GeomError::Degenerate(
                "interval addition overflows".to_owned(),
            ));
        }
        Ok(Self {
            lo: next_down(lo),
            hi: next_up(hi),
        })
    }

    pub(crate) fn subtract(self, other: Self) -> GeomResult<Self> {
        let lo = self.lo - other.hi;
        let hi = self.hi - other.lo;
        if !lo.is_finite() || !hi.is_finite() {
            return Err(GeomError::Degenerate(
                "interval subtraction overflows".to_owned(),
            ));
        }
        Ok(Self {
            lo: next_down(lo),
            hi: next_up(hi),
        })
    }

    fn multiply(self, other: Self) -> GeomResult<Self> {
        let products = [
            self.lo * other.lo,
            self.lo * other.hi,
            self.hi * other.lo,
            self.hi * other.hi,
        ];
        if products.iter().any(|value| !value.is_finite()) {
            return Err(GeomError::Degenerate(
                "interval multiplication overflows".to_owned(),
            ));
        }
        let lo = products.iter().copied().fold(Scalar::INFINITY, Scalar::min);
        let hi = products
            .iter()
            .copied()
            .fold(Scalar::NEG_INFINITY, Scalar::max);
        Ok(Self {
            lo: next_down(lo),
            hi: next_up(hi),
        })
    }

    #[cfg(test)]
    pub(crate) fn contains(self, value: Scalar) -> bool {
        self.lo <= value && value <= self.hi
    }

    fn midpoint(self, other: Self) -> GeomResult<Self> {
        let lo = self.lo * 0.5 + other.lo * 0.5;
        let hi = self.hi * 0.5 + other.hi * 0.5;
        if !lo.is_finite() || !hi.is_finite() {
            return Err(GeomError::Degenerate(
                "homogeneous subdivision overflows".to_owned(),
            ));
        }
        Ok(Self {
            lo: next_down(lo),
            hi: next_up(hi),
        })
    }

    pub(crate) fn divide(self, positive: Self) -> GeomResult<Self> {
        if positive.lo <= 0.0 || !positive.lo.is_finite() || !positive.hi.is_finite() {
            return Err(GeomError::InvalidInput(
                "certified rational bounds require positive finite weights".to_owned(),
            ));
        }
        let values = [
            self.lo / positive.lo,
            self.lo / positive.hi,
            self.hi / positive.lo,
            self.hi / positive.hi,
        ];
        if values.iter().any(|value| !value.is_finite()) {
            return Err(GeomError::Degenerate(
                "rational control enclosure is non-finite".to_owned(),
            ));
        }
        let lo = values.iter().copied().fold(Scalar::INFINITY, Scalar::min);
        let hi = values
            .iter()
            .copied()
            .fold(Scalar::NEG_INFINITY, Scalar::max);
        Ok(Self {
            lo: next_down(lo),
            hi: next_up(hi),
        })
    }
}

#[derive(Debug, Clone)]
pub(crate) struct HomogeneousPoint {
    pub(crate) numerator: [Interval; 3],
    pub(crate) weight: Interval,
}

impl HomogeneousPoint {
    pub(crate) fn blend(previous: &Self, current: &Self, alpha: Interval) -> GeomResult<Self> {
        if alpha.lo < 0.0 || alpha.hi > 1.0 {
            return Err(GeomError::Degenerate(
                "knot insertion blend escaped zero-to-one".to_owned(),
            ));
        }
        let one_minus = Interval::exact(1.0)?.subtract(alpha)?;
        let blend = |before: Interval, after: Interval| {
            alpha.multiply(after)?.add(one_minus.multiply(before)?)
        };
        Ok(Self {
            numerator: [
                blend(previous.numerator[0], current.numerator[0])?,
                blend(previous.numerator[1], current.numerator[1])?,
                blend(previous.numerator[2], current.numerator[2])?,
            ],
            weight: blend(previous.weight, current.weight)?,
        })
    }

    fn midpoint(&self, other: &Self) -> GeomResult<Self> {
        Ok(Self {
            numerator: [
                self.numerator[0].midpoint(other.numerator[0])?,
                self.numerator[1].midpoint(other.numerator[1])?,
                self.numerator[2].midpoint(other.numerator[2])?,
            ],
            weight: self.weight.midpoint(other.weight)?,
        })
    }

    pub(crate) fn euclidean(&self) -> GeomResult<[Interval; 3]> {
        Ok([
            self.numerator[0].divide(self.weight)?,
            self.numerator[1].divide(self.weight)?,
            self.numerator[2].divide(self.weight)?,
        ])
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Cell {
    pub(crate) controls: Vec<HomogeneousPoint>,
    pub(crate) start: Scalar,
    pub(crate) end: Scalar,
    pub(crate) depth: u16,
}

impl Cell {
    pub(crate) fn lower_bound(&self, target: [Scalar; 3], dimensions: usize) -> GeomResult<Scalar> {
        let (lo, hi) = self.coordinate_bounds()?;
        distance_to_box_lower(target, lo, hi, dimensions)
    }

    pub(crate) fn gap(&self, other: &Self, dimensions: usize) -> GeomResult<Scalar> {
        let (first_lo, first_hi) = self.coordinate_bounds()?;
        let (second_lo, second_hi) = other.coordinate_bounds()?;
        let mut sum = 0.0;
        for axis in 0..dimensions {
            let separation = if first_hi[axis] < second_lo[axis] {
                next_down(second_lo[axis] - first_hi[axis]).max(0.0)
            } else if second_hi[axis] < first_lo[axis] {
                next_down(first_lo[axis] - second_hi[axis]).max(0.0)
            } else {
                0.0
            };
            sum = next_down(sum + next_down(separation * separation)).max(0.0);
        }
        let distance = next_down(sum.sqrt()).max(0.0);
        if distance.is_finite() {
            Ok(distance)
        } else {
            Err(GeomError::Degenerate(
                "curve-pair lower distance bound is non-finite".to_owned(),
            ))
        }
    }

    fn coordinate_bounds(&self) -> GeomResult<([Scalar; 3], [Scalar; 3])> {
        let mut lo = [Scalar::INFINITY; 3];
        let mut hi = [Scalar::NEG_INFINITY; 3];
        for control in &self.controls {
            let point = control.euclidean()?;
            for axis in 0..3 {
                lo[axis] = lo[axis].min(point[axis].lo);
                hi[axis] = hi[axis].max(point[axis].hi);
            }
        }
        Ok((lo, hi))
    }

    pub(crate) fn midpoint_point(&self) -> GeomResult<HomogeneousPoint> {
        let mut level = self.controls.clone();
        while level.len() > 1 {
            let mut next = Vec::with_capacity(level.len() - 1);
            for pair in level.windows(2) {
                next.push(pair[0].midpoint(&pair[1])?);
            }
            level = next;
        }
        level
            .pop()
            .ok_or_else(|| GeomError::InvalidInput("empty Bézier control polygon".to_owned()))
    }

    pub(crate) fn split(&self) -> GeomResult<(Self, Self)> {
        let mut level = self.controls.clone();
        let mut left = Vec::with_capacity(level.len());
        let mut right = Vec::with_capacity(level.len());
        left.push(level[0].clone());
        right.push(level[level.len() - 1].clone());
        while level.len() > 1 {
            let mut next = Vec::with_capacity(level.len() - 1);
            for pair in level.windows(2) {
                next.push(pair[0].midpoint(&pair[1])?);
            }
            left.push(next[0].clone());
            right.push(next[next.len() - 1].clone());
            level = next;
        }
        right.reverse();
        let middle = self.start * 0.5 + self.end * 0.5;
        if !middle.is_finite() || middle <= self.start || middle >= self.end {
            return Err(GeomError::Degenerate(
                "parameter subdivision no longer advances".to_owned(),
            ));
        }
        Ok((
            Self {
                controls: left,
                start: self.start,
                end: middle,
                depth: self.depth + 1,
            },
            Self {
                controls: right,
                start: middle,
                end: self.end,
                depth: self.depth + 1,
            },
        ))
    }
}

fn distance_to_box_lower(
    target: [Scalar; 3],
    lo: [Scalar; 3],
    hi: [Scalar; 3],
    dimensions: usize,
) -> GeomResult<Scalar> {
    let mut sum = 0.0;
    for axis in 0..dimensions {
        let separation = if target[axis] < lo[axis] {
            next_down(lo[axis] - target[axis]).max(0.0)
        } else if target[axis] > hi[axis] {
            next_down(target[axis] - hi[axis]).max(0.0)
        } else {
            0.0
        };
        sum = next_down(sum + next_down(separation * separation)).max(0.0);
    }
    let distance = next_down(sum.sqrt()).max(0.0);
    if distance.is_finite() {
        Ok(distance)
    } else {
        Err(GeomError::Degenerate(
            "projection lower distance bound is non-finite".to_owned(),
        ))
    }
}

pub(crate) fn distance_to_point_interval_upper(
    target: [Scalar; 3],
    point: [Interval; 3],
    dimensions: usize,
) -> GeomResult<Scalar> {
    let mut sum = 0.0;
    for axis in 0..dimensions {
        let far = next_up(
            (point[axis].lo - target[axis])
                .abs()
                .max((point[axis].hi - target[axis]).abs()),
        );
        sum = next_up(sum + next_up(far * far));
    }
    let distance = next_up(sum.sqrt());
    if distance.is_finite() {
        Ok(distance)
    } else {
        Err(GeomError::Degenerate(
            "projection upper distance bound is non-finite".to_owned(),
        ))
    }
}

pub(crate) fn distance_between_point_intervals_upper(
    first: [Interval; 3],
    second: [Interval; 3],
    dimensions: usize,
) -> GeomResult<Scalar> {
    let mut sum = 0.0;
    for axis in 0..dimensions {
        let far = next_up(
            (first[axis].lo - second[axis].hi)
                .abs()
                .max((first[axis].hi - second[axis].lo).abs()),
        );
        sum = next_up(sum + next_up(far * far));
    }
    let distance = next_up(sum.sqrt());
    if distance.is_finite() {
        Ok(distance)
    } else {
        Err(GeomError::Degenerate(
            "curve-pair upper distance bound is non-finite".to_owned(),
        ))
    }
}

pub(crate) fn representative_distance(
    point: [Scalar; 3],
    target: [Scalar; 3],
    dimensions: usize,
) -> GeomResult<Scalar> {
    let distance = (0..dimensions)
        .map(|axis| (point[axis] - target[axis]).powi(2))
        .sum::<Scalar>()
        .sqrt();
    if distance.is_finite() {
        Ok(distance)
    } else {
        Err(GeomError::Degenerate(
            "projection representative distance is non-finite".to_owned(),
        ))
    }
}

pub(crate) fn next_up(value: Scalar) -> Scalar {
    if value.is_nan() || value == Scalar::INFINITY {
        return value;
    }
    if value == 0.0 {
        return Scalar::from_bits(1);
    }
    let bits = value.to_bits();
    Scalar::from_bits(if value > 0.0 { bits + 1 } else { bits - 1 })
}

fn next_down(value: Scalar) -> Scalar {
    if value.is_nan() || value == Scalar::NEG_INFINITY {
        return value;
    }
    if value == 0.0 {
        return -Scalar::from_bits(1);
    }
    let bits = value.to_bits();
    Scalar::from_bits(if value > 0.0 { bits - 1 } else { bits + 1 })
}
