//! Typed refusals for linear intersection.
//!
//! Every failure names which operand caused it. A caller repairing input data
//! cannot act on "something was invalid".

/// Which operand a diagnostic refers to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InputSide {
    /// The first operand.
    Left,
    /// The second operand.
    Right,
}

/// Why a linear intersection could not be answered.
///
/// These are refusals, not results: none of them may be interpreted as
/// "no intersection". A disjoint configuration is a successful classification.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LinearIntersectionError {
    /// An operand carried a non-finite coordinate or direction component.
    NonFiniteInput {
        /// Operand at fault.
        side: InputSide,
    },
    /// An operand's direction was zero, so it does not define a line.
    DegenerateDirection {
        /// Operand at fault.
        side: InputSide,
    },
    /// An intermediate value left the finite range, so no sign is provable.
    ArithmeticOverflow,
}

impl core::fmt::Display for LinearIntersectionError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NonFiniteInput { side } => write!(f, "{side:?} operand has non-finite input"),
            Self::DegenerateDirection { side } => {
                write!(f, "{side:?} operand has a zero-length direction")
            }
            Self::ArithmeticOverflow => {
                f.write_str("intersection arithmetic left the finite range")
            }
        }
    }
}

impl std::error::Error for LinearIntersectionError {}
