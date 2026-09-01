//! Diagnosis values; diagnosis never mutates geometry.

/// Defect class observed in mesh or exact topology.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DefectKind {
    /// Edge used by other than two faces where manifoldness is required.
    NonManifoldEdge,
    /// Adjacent orientation disagrees.
    InconsistentOrientation,
    /// Coincident vertices are represented separately.
    DuplicateVertex,
    /// Zero-area/zero-length entity.
    DegenerateElement,
    /// Shell is not closed.
    OpenShell,
    /// Shape intersects itself.
    SelfIntersection,
}

/// One located defect.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Defect {
    /// Defect class.
    pub kind: DefectKind,
    /// Representation-specific stable index where available.
    pub element: Option<u32>,
    /// Additional diagnostic context.
    pub detail: Option<String>,
}

/// Immutable diagnosis report.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Diagnosis {
    /// Defects in deterministic discovery order.
    pub defects: Vec<Defect>,
}

impl Diagnosis {
    /// Whether no defects were found.
    pub fn is_clean(&self) -> bool {
        self.defects.is_empty()
    }

    /// Whether a robust boolean precondition is violated.
    pub fn blocks_boolean(&self) -> bool {
        self.defects.iter().any(|defect| {
            matches!(
                defect.kind,
                DefectKind::NonManifoldEdge
                    | DefectKind::InconsistentOrientation
                    | DefectKind::OpenShell
                    | DefectKind::SelfIntersection
            )
        })
    }
}
