//! Small operation values shared by representation and algorithm contracts.

/// Boolean set operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BooleanOperator {
    Union,
    Intersection,
    Difference,
}
