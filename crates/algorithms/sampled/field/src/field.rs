//! Compact row-major grid of layered cells plus its sampling evidence.

use axiolid_core::Tolerance;

use crate::{FieldConfig, LayeredCell, LayeredFieldError};

/// Structured facts about one sampling run.
///
/// Evidence is reported, never used to silently alter the result.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct FieldEvidence {
    /// Cells visited by the sampler.
    pub cells_sampled: usize,
    /// Surface crossings stored across all cells.
    pub surface_hits: usize,
    /// Occupancy spans stored across all cells.
    pub occupancy_spans: usize,
    /// Cells holding no layer at all.
    pub empty_cells: usize,
    /// Cells holding more than one surface crossing.
    pub multi_layer_cells: usize,
    /// Triangles skipped because they are parallel to the sampling direction.
    pub parallel_triangles_skipped: usize,
    /// Crossings discarded because they fell outside the local `w` bounds.
    pub out_of_bounds_hits: usize,
    /// Crossings that landed on a triangle edge or vertex within tolerance.
    pub boundary_contacts: usize,
    /// Coincident crossings collapsed because adjacent facets share an edge.
    ///
    /// A sampling line through a shared edge meets both facets, but the two
    /// reports describe one surface. They are merged and counted here rather
    /// than stored twice or dropped silently.
    pub coincident_hits_merged: usize,
    /// Triangles rejected as degenerate before sampling.
    pub degenerate_triangles: usize,
}

/// Deterministic row-major field of layered cells.
#[derive(Debug, Clone, PartialEq)]
pub struct LayeredField {
    width: usize,
    height: usize,
    cells: Vec<LayeredCell>,
    evidence: FieldEvidence,
}

impl LayeredField {
    /// Allocate an empty field from a validated configuration.
    pub fn with_config(config: &FieldConfig) -> Result<Self, LayeredFieldError> {
        let (width, height) = config.dimensions();
        Self::empty(width, height)
    }

    /// Allocate an empty field with explicit dimensions.
    pub fn empty(width: usize, height: usize) -> Result<Self, LayeredFieldError> {
        let cell_count = width
            .checked_mul(height)
            .ok_or(LayeredFieldError::InvalidDimensions)?;
        if cell_count == 0 {
            return Err(LayeredFieldError::InvalidDimensions);
        }
        Ok(Self {
            width,
            height,
            cells: vec![LayeredCell::empty(); cell_count],
            evidence: FieldEvidence::default(),
        })
    }

    /// Cell dimensions.
    pub const fn dimensions(&self) -> (usize, usize) {
        (self.width, self.height)
    }

    /// Number of cells in row-major storage.
    pub fn cell_count(&self) -> usize {
        self.cells.len()
    }

    /// Row-major address for `(x, y)`, or `None` when outside the field.
    pub fn linear_index(&self, x: usize, y: usize) -> Option<usize> {
        (x < self.width && y < self.height).then_some(y * self.width + x)
    }

    /// Borrow the cell at `(x, y)`.
    pub fn cell(&self, x: usize, y: usize) -> Option<&LayeredCell> {
        self.linear_index(x, y).map(|index| &self.cells[index])
    }

    /// Cells in row-major order.
    pub fn cells(&self) -> &[LayeredCell] {
        &self.cells
    }

    /// Structured sampling facts.
    pub const fn evidence(&self) -> FieldEvidence {
        self.evidence
    }

    pub(crate) fn set_cell(&mut self, index: usize, cell: LayeredCell) {
        self.cells[index] = cell;
    }

    pub(crate) fn set_evidence(&mut self, evidence: FieldEvidence) {
        self.evidence = evidence;
    }

    /// Derive occupancy spans in every cell from its crossing sequence.
    ///
    /// Requires closed, consistently wound input: an unbalanced column is
    /// reported rather than repaired.
    pub fn derive_occupancy(&self, tolerance: Tolerance) -> Result<Self, LayeredFieldError> {
        let mut derived = self.clone();
        let mut evidence = self.evidence;
        evidence.occupancy_spans = 0;
        for (index, cell) in self.cells.iter().enumerate() {
            let updated = cell.derive_occupancy(tolerance)?;
            evidence.occupancy_spans += updated.occupancy().len();
            derived.cells[index] = updated;
        }
        derived.evidence = evidence;
        Ok(derived)
    }
}
