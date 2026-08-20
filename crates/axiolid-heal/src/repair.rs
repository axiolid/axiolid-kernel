//! Explicit repair plans and reports.

/// One opt-in repair. There is deliberately no `All` variant.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RepairAction {
    /// Merge vertices within tolerance.
    WeldVertices,
    /// Make connected-face orientation consistent.
    UnifyOrientation,
    /// Remove degenerate elements.
    DropDegenerateElements,
}

/// Ordered caller-approved repair plan.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RepairPlan {
    /// Repairs to attempt in order.
    pub actions: Vec<RepairAction>,
}

/// Audit report returned with repaired geometry.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RepairReport {
    /// Repairs actually applied.
    pub applied: Vec<RepairAction>,
    /// Repairs requested but not applicable.
    pub skipped: Vec<RepairAction>,
}
