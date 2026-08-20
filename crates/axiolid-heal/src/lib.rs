#![forbid(unsafe_code)]

//! Explicit geometry diagnosis and opt-in repair.
//!
//! Healing never runs inside another algorithm. Callers diagnose first, choose
//! a narrow [`RepairPlan`], and retain the resulting [`RepairReport`] for audit.

pub mod diagnosis;
pub mod repair;
pub mod traits;

pub use diagnosis::{Defect, DefectKind, Diagnosis};
pub use repair::{RepairAction, RepairPlan, RepairReport};
pub use traits::{Diagnose, Repair};
