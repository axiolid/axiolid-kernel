/// Arithmetic precision accepted or required by an operation.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Precision {
    /// IEEE single precision.
    F32,
    /// IEEE double precision.
    F64,
    /// Deliberate mixed-precision path with documented error bounds.
    Mixed,
    /// Exact arithmetic: the result carries no rounding error at all.
    Exact,
}
