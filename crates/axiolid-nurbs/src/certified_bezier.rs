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

    pub(crate) fn bounds(lo: Scalar, hi: Scalar) -> GeomResult<Self> {
        if !lo.is_finite() || !hi.is_finite() || lo > hi {
            return Err(GeomError::InvalidInput(
                "invalid certified interval bounds".to_owned(),
            ));
        }
        Ok(Self { lo, hi })
    }

    pub(crate) fn hull(values: impl IntoIterator<Item = Self>) -> GeomResult<Self> {
        let mut lo = Scalar::INFINITY;
        let mut hi = Scalar::NEG_INFINITY;
        for value in values {
            lo = lo.min(value.lo);
            hi = hi.max(value.hi);
        }
        Self::bounds(lo, hi)
    }

    pub(crate) const fn lower(self) -> Scalar {
        self.lo
    }

    pub(crate) const fn upper(self) -> Scalar {
        self.hi
    }

    pub(crate) const fn contains_zero(self) -> bool {
        self.lo <= 0.0 && self.hi >= 0.0
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

    pub(crate) fn add(self, other: Self) -> GeomResult<Self> {
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

    pub(crate) fn multiply(self, other: Self) -> GeomResult<Self> {
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

    pub(crate) fn absolute_lower_bound(self) -> Scalar {
        if self.lo > 0.0 {
            self.lo
        } else if self.hi < 0.0 {
            -self.hi
        } else {
            0.0
        }
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

    pub(crate) fn divide_nonzero(self, denominator: Self) -> GeomResult<Self> {
        if denominator.contains_zero() {
            return Err(GeomError::Degenerate(
                "interval division denominator contains zero".to_owned(),
            ));
        }
        if denominator.hi < 0.0 {
            let negative = Interval::exact(-1.0)?;
            return self
                .multiply(negative)?
                .divide(denominator.multiply(negative)?);
        }
        self.divide(denominator)
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
    pub(crate) fn coordinate_intervals(&self) -> GeomResult<[Interval; 3]> {
        let controls = self
            .controls
            .iter()
            .map(HomogeneousPoint::euclidean)
            .collect::<GeomResult<Vec<_>>>()?;
        Ok([
            Interval::hull(controls.iter().map(|point| point[0]))?,
            Interval::hull(controls.iter().map(|point| point[1]))?,
            Interval::hull(controls.iter().map(|point| point[2]))?,
        ])
    }

    pub(crate) fn derivative_intervals(&self) -> GeomResult<[Interval; 3]> {
        let degree =
            self.controls.len().checked_sub(1).ok_or_else(|| {
                GeomError::InvalidInput("empty Bézier control polygon".to_owned())
            })?;
        if degree == 0 {
            return Ok([Interval::exact(0.0)?; 3]);
        }
        let span = Interval::exact(self.end)?.subtract(Interval::exact(self.start)?)?;
        let scale = Interval::exact(degree as Scalar)?.divide(span)?;
        let derivatives = self
            .controls
            .windows(2)
            .map(|pair| {
                Ok(HomogeneousPoint {
                    numerator: [
                        pair[1].numerator[0]
                            .subtract(pair[0].numerator[0])?
                            .multiply(scale)?,
                        pair[1].numerator[1]
                            .subtract(pair[0].numerator[1])?
                            .multiply(scale)?,
                        pair[1].numerator[2]
                            .subtract(pair[0].numerator[2])?
                            .multiply(scale)?,
                    ],
                    weight: pair[1].weight.subtract(pair[0].weight)?.multiply(scale)?,
                })
            })
            .collect::<GeomResult<Vec<_>>>()?;
        let weight = Interval::hull(self.controls.iter().map(|control| control.weight))?;
        let weight_prime = Interval::hull(derivatives.iter().map(|control| control.weight))?;
        let denominator = weight.multiply(weight)?;
        let mut result = [Interval::exact(0.0)?; 3];
        for (axis, value) in result.iter_mut().enumerate() {
            let numerator =
                Interval::hull(self.controls.iter().map(|control| control.numerator[axis]))?;
            let numerator_prime =
                Interval::hull(derivatives.iter().map(|control| control.numerator[axis]))?;
            *value = numerator_prime
                .multiply(weight)?
                .subtract(numerator.multiply(weight_prime)?)?
                .divide(denominator)?;
        }
        Ok(result)
    }

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

    pub(crate) fn restrict(&self, start: Scalar, end: Scalar) -> GeomResult<Self> {
        if !start.is_finite()
            || !end.is_finite()
            || start < self.start
            || end > self.end
            || start >= end
        {
            return Err(GeomError::InvalidInput(
                "Bézier restriction must be a finite nonempty subinterval".to_owned(),
            ));
        }
        let mut restricted = if start > self.start {
            self.split_at(start)?.1
        } else {
            self.clone()
        };
        if end < restricted.end {
            restricted = restricted.split_at(end)?.0;
        }
        restricted.depth = self
            .depth
            .checked_add(1)
            .ok_or_else(|| GeomError::Degenerate("Bézier restriction depth overflow".to_owned()))?;
        Ok(restricted)
    }

    pub(crate) fn split(&self) -> GeomResult<(Self, Self)> {
        let middle = self.start * 0.5 + self.end * 0.5;
        if middle <= self.start || middle >= self.end {
            return Err(no_split_error(self, middle));
        }
        self.split_with_alpha(middle, Interval::exact(0.5)?)
    }

    fn split_at(&self, parameter: Scalar) -> GeomResult<(Self, Self)> {
        if !parameter.is_finite() || parameter <= self.start || parameter >= self.end {
            return Err(no_split_error(self, parameter));
        }
        let span = Interval::exact(self.end)?.subtract(Interval::exact(self.start)?)?;
        let mut alpha = Interval::exact(parameter)?
            .subtract(Interval::exact(self.start)?)?
            .divide(span)?;
        alpha.lo = alpha.lo.max(0.0);
        alpha.hi = alpha.hi.min(1.0);
        self.split_with_alpha(parameter, alpha)
    }

    fn split_with_alpha(&self, parameter: Scalar, alpha: Interval) -> GeomResult<(Self, Self)> {
        let mut level = self.controls.clone();
        let mut left = Vec::with_capacity(level.len());
        let mut right = Vec::with_capacity(level.len());
        left.push(level[0].clone());
        right.push(level[level.len() - 1].clone());
        while level.len() > 1 {
            let mut next = Vec::with_capacity(level.len() - 1);
            for pair in level.windows(2) {
                next.push(HomogeneousPoint::blend(&pair[0], &pair[1], alpha)?);
            }
            left.push(next[0].clone());
            right.push(next[next.len() - 1].clone());
            level = next;
        }
        right.reverse();
        let depth = self
            .depth
            .checked_add(1)
            .ok_or_else(|| GeomError::Degenerate("Bézier subdivision depth overflow".to_owned()))?;
        Ok((
            Self {
                controls: left,
                start: self.start,
                end: parameter,
                depth,
            },
            Self {
                controls: right,
                start: parameter,
                end: self.end,
                depth,
            },
        ))
    }
}

fn no_split_error(cell: &Cell, parameter: Scalar) -> GeomError {
    GeomError::Degenerate(format!(
        "parameter subdivision no longer advances: [{}, {}] at {}",
        cell.start, cell.end, parameter
    ))
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
