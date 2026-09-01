//! Explicit sampling frame, bounds, cell size, tolerance, and resource budget.
//!
//! Nothing here assumes a world axis. The frame's local `z` is the layering
//! axis; `x` and `y` address cells. A caller that wants world Z-up passes the
//! identity frame explicitly, and a caller slicing a wall passes its own frame.

use axiolid_core::{Frame3, Interval, Point3, Scalar, Tolerance};

use crate::LayeredFieldError;

/// Caller-owned allocation limits. There is no built-in cap: a library must not
/// decide how much memory an application may spend.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FieldResourceBudget {
    /// Maximum number of addressable `(x, y)` cells.
    pub max_cells: usize,
    /// Maximum number of stored layers (surface hits plus occupancy intervals).
    pub max_intervals: usize,
}

impl FieldResourceBudget {
    /// Construct an explicit budget.
    pub const fn new(max_cells: usize, max_intervals: usize) -> Self {
        Self {
            max_cells,
            max_intervals,
        }
    }
}

/// Local-frame sampling box with `min` strictly below `max` on every axis.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FieldBounds {
    min: Point3,
    max: Point3,
}

impl FieldBounds {
    /// Validate a non-degenerate finite local box.
    pub fn new(min: Point3, max: Point3) -> Result<Self, LayeredFieldError> {
        if !min.is_finite() || !max.is_finite() {
            return Err(LayeredFieldError::InvalidBounds);
        }
        if min.x >= max.x || min.y >= max.y || min.z >= max.z {
            return Err(LayeredFieldError::InvalidBounds);
        }
        Ok(Self { min, max })
    }

    /// Lower corner in local coordinates.
    pub const fn min(self) -> Point3 {
        self.min
    }

    /// Upper corner in local coordinates.
    pub const fn max(self) -> Point3 {
        self.max
    }

    /// Layering-axis span as an oriented interval.
    pub fn normal_span(self) -> Interval {
        Interval::new(self.min.z, self.max.z)
    }
}

/// Validated configuration for one layered field.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FieldConfig {
    frame: Frame3,
    bounds: FieldBounds,
    cell_size: Scalar,
    tolerance: Tolerance,
    budget: FieldResourceBudget,
    width: usize,
    height: usize,
}

impl FieldConfig {
    /// Validate a frame, bounds, cell size, tolerance, and budget together.
    ///
    /// The frame must be finite, unit-length on each axis, mutually orthogonal
    /// within the angular tolerance, and right-handed. Grid dimensions are
    /// derived deterministically by ceiling division so a partial trailing cell
    /// is retained rather than silently dropped.
    pub fn new(
        frame: Frame3,
        bounds: FieldBounds,
        cell_size: Scalar,
        tolerance: Tolerance,
        budget: FieldResourceBudget,
    ) -> Result<Self, LayeredFieldError> {
        if !cell_size.is_finite() || cell_size <= 0.0 {
            return Err(LayeredFieldError::InvalidCellSize);
        }
        if !frame_is_orthonormal(frame, tolerance) {
            return Err(LayeredFieldError::InvalidFrame);
        }
        let span = bounds.max() - bounds.min();
        let width = ceil_cells(span.x, cell_size)?;
        let height = ceil_cells(span.y, cell_size)?;
        let cells = width
            .checked_mul(height)
            .ok_or(LayeredFieldError::CellBudgetExceeded)?;
        if cells == 0 {
            return Err(LayeredFieldError::InvalidDimensions);
        }
        if cells > budget.max_cells {
            return Err(LayeredFieldError::CellBudgetExceeded);
        }
        Ok(Self {
            frame,
            bounds,
            cell_size,
            tolerance,
            budget,
            width,
            height,
        })
    }

    /// Sampling frame supplied by the caller.
    pub const fn frame(&self) -> Frame3 {
        self.frame
    }

    /// Local sampling box.
    pub const fn bounds(&self) -> FieldBounds {
        self.bounds
    }

    /// Edge length of one square cell in local units.
    pub const fn cell_size(&self) -> Scalar {
        self.cell_size
    }

    /// Explicit tolerance policy for this field.
    pub const fn tolerance(&self) -> Tolerance {
        self.tolerance
    }

    /// Caller-owned budget.
    pub const fn budget(&self) -> FieldResourceBudget {
        self.budget
    }

    /// Deterministic `(width, height)` cell dimensions.
    pub const fn dimensions(&self) -> (usize, usize) {
        (self.width, self.height)
    }

    /// World-space centre of cell `(x, y)` on the `w = 0` local plane.
    pub fn cell_center(&self, x: usize, y: usize) -> Point3 {
        let local_x = self.bounds.min().x + (x as Scalar + 0.5) * self.cell_size;
        let local_y = self.bounds.min().y + (y as Scalar + 0.5) * self.cell_size;
        self.frame.origin + self.frame.x * local_x + self.frame.y * local_y
    }

    /// World-space point for a local `(x, y)` cell centre at layer coordinate `w`.
    pub fn sample_point(&self, x: usize, y: usize, w: Scalar) -> Point3 {
        self.cell_center(x, y) + self.frame.z * w
    }
}

fn ceil_cells(span: Scalar, cell_size: Scalar) -> Result<usize, LayeredFieldError> {
    let count = (span / cell_size).ceil();
    if !count.is_finite() || count <= 0.0 || count > usize::MAX as Scalar {
        return Err(LayeredFieldError::CellBudgetExceeded);
    }
    Ok(count as usize)
}

fn frame_is_orthonormal(frame: Frame3, tolerance: Tolerance) -> bool {
    let unit = tolerance.angular().max(Scalar::EPSILON * 8.0);
    frame.origin.is_finite()
        && frame.x.is_finite()
        && frame.y.is_finite()
        && frame.z.is_finite()
        && (frame.x.length_squared() - 1.0).abs() <= unit
        && (frame.y.length_squared() - 1.0).abs() <= unit
        && (frame.z.length_squared() - 1.0).abs() <= unit
        && frame.x.dot(frame.y).abs() <= unit
        && frame.y.dot(frame.z).abs() <= unit
        && frame.z.dot(frame.x).abs() <= unit
        && frame.x.cross(frame.y).dot(frame.z) > 0.0
}
