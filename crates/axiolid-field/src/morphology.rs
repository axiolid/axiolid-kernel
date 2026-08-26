//! Planar masks and metric morphology over a layered field.
//!
//! Morphology here is a geometric set operation on a 2D support mask. The
//! caller chooses which channel forms the mask and what radius to inflate by;
//! this module attaches no meaning such as "obstacle", "walkable", or "clear". NOT-A-VERDICT

use axiolid_core::Scalar;

use crate::{FieldConfig, LayeredField, LayeredFieldError};

/// Which channel of a cell contributes to a planar mask.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldChannel {
    /// A cell is set when it holds at least one surface crossing.
    SurfacePresence,
    /// A cell is set when it holds at least one occupancy span.
    OccupancyPresence,
}

/// A dense row-major boolean mask aligned with a field's cell grid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanarMask {
    width: usize,
    height: usize,
    bits: Vec<bool>,
}

impl PlanarMask {
    /// Construct an all-false mask.
    pub fn empty(width: usize, height: usize) -> Result<Self, LayeredFieldError> {
        let count = width
            .checked_mul(height)
            .ok_or(LayeredFieldError::InvalidDimensions)?;
        if count == 0 {
            return Err(LayeredFieldError::InvalidDimensions);
        }
        Ok(Self {
            width,
            height,
            bits: vec![false; count],
        })
    }

    /// Derive a mask from one channel of a field.
    pub fn from_field(field: &LayeredField, channel: FieldChannel) -> Self {
        let (width, height) = field.dimensions();
        let bits = field
            .cells()
            .iter()
            .map(|cell| match channel {
                FieldChannel::SurfacePresence => !cell.surfaces().is_empty(),
                FieldChannel::OccupancyPresence => !cell.occupancy().is_empty(),
            })
            .collect();
        Self {
            width,
            height,
            bits,
        }
    }

    /// Mask dimensions.
    pub const fn dimensions(&self) -> (usize, usize) {
        (self.width, self.height)
    }

    /// Number of set cells.
    pub fn count(&self) -> usize {
        self.bits.iter().filter(|bit| **bit).count()
    }

    /// Read a cell, or `None` when outside the mask.
    pub fn get(&self, x: usize, y: usize) -> Option<bool> {
        self.index(x, y).map(|index| self.bits[index])
    }

    /// Write a cell.
    pub fn set(&mut self, x: usize, y: usize, value: bool) -> Result<(), LayeredFieldError> {
        let index = self
            .index(x, y)
            .ok_or(LayeredFieldError::NodeOutsideField)?;
        self.bits[index] = value;
        Ok(())
    }

    /// Set-complement.
    pub fn inverted(&self) -> Self {
        Self {
            width: self.width,
            height: self.height,
            bits: self.bits.iter().map(|bit| !bit).collect(),
        }
    }

    /// Set-intersection with an identically sized mask.
    pub fn intersect(&self, other: &Self) -> Result<Self, LayeredFieldError> {
        if self.width != other.width || self.height != other.height {
            return Err(LayeredFieldError::DimensionMismatch);
        }
        Ok(Self {
            width: self.width,
            height: self.height,
            bits: self
                .bits
                .iter()
                .zip(&other.bits)
                .map(|(left, right)| *left && *right)
                .collect(),
        })
    }

    /// Metric dilation by `radius` in local units.
    ///
    /// The radius is converted to a cell count by ceiling division on the
    /// configured cell size, so the result never under-covers the requested
    /// distance. A zero radius is the identity.
    pub fn dilate(&self, config: &FieldConfig, radius: Scalar) -> Result<Self, LayeredFieldError> {
        self.morph(config, radius, true)
    }

    /// Metric erosion by `radius` in local units.
    pub fn erode(&self, config: &FieldConfig, radius: Scalar) -> Result<Self, LayeredFieldError> {
        self.morph(config, radius, false)
    }

    fn morph(
        &self,
        config: &FieldConfig,
        radius: Scalar,
        dilate: bool,
    ) -> Result<Self, LayeredFieldError> {
        if !radius.is_finite() || radius < 0.0 {
            return Err(LayeredFieldError::InvalidEnvelope);
        }
        let steps = radius_in_cells(config, radius)?;
        if steps == 0 {
            return Ok(self.clone());
        }
        let reach = radius / config.cell_size();
        let reach_squared = reach * reach;
        let mut out = Self::empty(self.width, self.height)?;
        for y in 0..self.height {
            for x in 0..self.width {
                let mut value = !dilate;
                'window: for dy in -(steps as isize)..=(steps as isize) {
                    for dx in -(steps as isize)..=(steps as isize) {
                        // Euclidean structuring element, not a square kernel.
                        if (dx * dx + dy * dy) as Scalar > reach_squared {
                            continue;
                        }
                        let nx = x as isize + dx;
                        let ny = y as isize + dy;
                        // Erosion treats outside-the-field as unset, so a mask
                        // touching the border erodes inward rather than
                        // pretending the world continues.
                        let neighbor = if nx < 0
                            || ny < 0
                            || nx as usize >= self.width
                            || ny as usize >= self.height
                        {
                            false
                        } else {
                            self.bits[ny as usize * self.width + nx as usize]
                        };
                        if dilate && neighbor {
                            value = true;
                            break 'window;
                        }
                        if !dilate && !neighbor {
                            value = false;
                            break 'window;
                        }
                    }
                }
                out.bits[y * self.width + x] = value;
            }
        }
        Ok(out)
    }

    /// Label 4-connected components of set cells.
    ///
    /// Labels are assigned in row-major discovery order, so the labelling is
    /// stable across runs.
    pub fn connected_components(&self) -> ComponentLabels {
        let mut labels = vec![None; self.bits.len()];
        let mut next = 0usize;
        let mut stack = Vec::new();
        for start in 0..self.bits.len() {
            if !self.bits[start] || labels[start].is_some() {
                continue;
            }
            let label = next;
            next += 1;
            labels[start] = Some(label);
            stack.push(start);
            while let Some(index) = stack.pop() {
                let x = index % self.width;
                let y = index / self.width;
                for (nx, ny) in neighbors(x, y, self.width, self.height) {
                    let neighbor = ny * self.width + nx;
                    if self.bits[neighbor] && labels[neighbor].is_none() {
                        labels[neighbor] = Some(label);
                        stack.push(neighbor);
                    }
                }
            }
        }
        ComponentLabels {
            width: self.width,
            height: self.height,
            labels,
            count: next,
        }
    }

    fn index(&self, x: usize, y: usize) -> Option<usize> {
        (x < self.width && y < self.height).then_some(y * self.width + x)
    }
}

/// Component identifiers for every cell of a mask.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComponentLabels {
    width: usize,
    height: usize,
    labels: Vec<Option<usize>>,
    count: usize,
}

impl ComponentLabels {
    /// Number of distinct components.
    pub const fn count(&self) -> usize {
        self.count
    }

    /// Component containing `(x, y)`, or `None` for an unset or outside cell.
    pub fn label(&self, x: usize, y: usize) -> Option<usize> {
        if x >= self.width || y >= self.height {
            return None;
        }
        self.labels[y * self.width + x]
    }

    /// Whether two cells share one component.
    pub fn same_component(&self, from: (usize, usize), to: (usize, usize)) -> bool {
        match (self.label(from.0, from.1), self.label(to.0, to.1)) {
            (Some(left), Some(right)) => left == right,
            _ => false,
        }
    }
}

pub(crate) fn radius_in_cells(
    config: &FieldConfig,
    radius: Scalar,
) -> Result<usize, LayeredFieldError> {
    if !radius.is_finite() || radius < 0.0 {
        return Err(LayeredFieldError::InvalidEnvelope);
    }
    let steps = (radius / config.cell_size()).ceil();
    if !steps.is_finite() || steps > usize::MAX as Scalar {
        return Err(LayeredFieldError::InvalidEnvelope);
    }
    Ok(steps as usize)
}

pub(crate) fn neighbors(
    x: usize,
    y: usize,
    width: usize,
    height: usize,
) -> impl Iterator<Item = (usize, usize)> {
    let mut out = Vec::with_capacity(4);
    if x > 0 {
        out.push((x - 1, y));
    }
    if y > 0 {
        out.push((x, y - 1));
    }
    if x + 1 < width {
        out.push((x + 1, y));
    }
    if y + 1 < height {
        out.push((x, y + 1));
    }
    out.into_iter()
}
