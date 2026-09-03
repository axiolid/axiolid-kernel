//! Shared conformance checks for portable mesh-section providers.

use core::fmt;

use axiolid_contracts::ExecutionOptions;
use axiolid_core::{Frame3, Point3, Tolerance, Vec3};
use axiolid_mesh::TriMesh;

use crate::{MeshPlaneSection, SectionLimits};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConformanceFailure {
    ReturnedError(String),
    EmptyCentralSection,
    IncorrectEvidence,
    NonDeterministic,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ConformanceReport {
    pub failures: Vec<ConformanceFailure>,
}

impl ConformanceReport {
    pub fn is_success(&self) -> bool {
        self.failures.is_empty()
    }
}

impl fmt::Display for ConformanceReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_success() {
            write!(f, "conformant")
        } else {
            write!(f, "{:?}", self.failures)
        }
    }
}

pub struct ConformanceSuite;

impl ConformanceSuite {
    pub fn run(provider: &dyn MeshPlaneSection) -> ConformanceReport {
        let mut report = ConformanceReport::default();
        let mesh = unit_cube();
        let frame = Frame3 {
            origin: Point3::new(0.0, 0.0, 0.5),
            x: Vec3::X,
            y: Vec3::Y,
            z: Vec3::Z,
        };
        let limits = SectionLimits::new(8, 12, 32, 4);
        let options = ExecutionOptions::new(Tolerance::new(1e-9, 1e-9).expect("valid tolerance"));
        let first = provider.section(&mesh, frame, limits, &options);
        let second = provider.section(&mesh, frame, limits, &options);
        match (first, second) {
            (Ok(a), Ok(b)) => {
                if a.contours.is_empty() {
                    report
                        .failures
                        .push(ConformanceFailure::EmptyCentralSection);
                }
                if a.evidence.source_triangles != mesh.triangles().len()
                    || !a.evidence.is_derived_from_input_mesh()
                {
                    report.failures.push(ConformanceFailure::IncorrectEvidence);
                }
                if a != b {
                    report.failures.push(ConformanceFailure::NonDeterministic);
                }
            }
            (Err(error), _) | (_, Err(error)) => report
                .failures
                .push(ConformanceFailure::ReturnedError(error.to_string())),
        }
        report
    }
}

fn unit_cube() -> TriMesh {
    TriMesh::new(
        vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(1.0, 1.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
            Point3::new(0.0, 0.0, 1.0),
            Point3::new(1.0, 0.0, 1.0),
            Point3::new(1.0, 1.0, 1.0),
            Point3::new(0.0, 1.0, 1.0),
        ],
        vec![
            0, 2, 1, 0, 3, 2, 4, 5, 6, 4, 6, 7, 0, 1, 5, 0, 5, 4, 1, 2, 6, 1, 6, 5, 2, 3, 7, 2, 7,
            6, 3, 0, 4, 3, 4, 7,
        ],
    )
}
