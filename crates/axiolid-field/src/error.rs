//! Structured layered-field failures.
//!
//! Every variant names a geometric or budget fact. None of them encodes an
//! application rule, a compliance verdict, or a domain interpretation. NOT-A-VERDICT

use core::fmt;

/// Invalid layered-field input, malformed topology, or exhausted caller budget.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum LayeredFieldError {
    /// An interval endpoint was non-finite or did not satisfy `start < end`.
    InvalidInterval,
    /// Two occupancy intervals overlap or touch; merging would change topology.
    NonDisjointIntervals,
    /// The requested local-grid dimensions exceed the caller budget.
    CellBudgetExceeded,
    /// Frame axes are non-finite, non-orthonormal, or left-handed.
    InvalidFrame,
    /// Local bounds are non-finite or degenerate on some axis.
    InvalidBounds,
    /// Cell size is non-finite or not strictly positive.
    InvalidCellSize,
    /// Input geometry carried a non-finite coordinate.
    NonFiniteGeometry,
    /// Field dimensions cannot be represented or contain no cells.
    InvalidDimensions,
    /// Stored layers exceeded the caller-owned layer budget.
    SampleBudgetExceeded,
    /// A cell's crossing sequence is not an alternating enter/exit sequence.
    UnbalancedCrossings,
    /// A derived occupancy span collapsed to within the linear tolerance.
    DegenerateOccupancy,
    /// A traversal envelope value was non-finite or negative.
    InvalidEnvelope,
    /// A referenced cell or support level does not exist in the field.
    NodeOutsideField,
    /// Two fields or masks do not share identical dimensions.
    DimensionMismatch,
}

impl fmt::Display for LayeredFieldError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            Self::InvalidInterval => "interval endpoints must be finite with start < end",
            Self::NonDisjointIntervals => "occupancy intervals must be strictly disjoint",
            Self::CellBudgetExceeded => "cell count exceeds the caller budget",
            Self::InvalidFrame => "frame axes must be finite, orthonormal, and right-handed",
            Self::InvalidBounds => "local bounds must be finite and non-degenerate",
            Self::InvalidCellSize => "cell size must be finite and strictly positive",
            Self::NonFiniteGeometry => "input geometry contains a non-finite coordinate",
            Self::InvalidDimensions => "field dimensions are empty or unrepresentable",
            Self::SampleBudgetExceeded => "stored layers exceed the caller budget",
            Self::UnbalancedCrossings => "crossings do not alternate enter/exit",
            Self::DegenerateOccupancy => "derived occupancy span collapsed within tolerance",
            Self::InvalidEnvelope => "envelope values must be finite and non-negative",
            Self::NodeOutsideField => "cell or support level is outside the field",
            Self::DimensionMismatch => "operands do not share identical dimensions",
        };
        f.write_str(text)
    }
}

impl std::error::Error for LayeredFieldError {}
