#![forbid(unsafe_code)]
//! Portable mesh plane-section operation contract and evidence.

mod contract;

pub use contract::{
    MeshPlaneSection, SectionContour, SectionEvidence, SectionLimits, SectionOutcome, SectionSource,
};

pub const CAPABILITY_ID: axiolid_contracts::CapabilityId =
    axiolid_contracts::capability_ids::MESH_SECTION;
