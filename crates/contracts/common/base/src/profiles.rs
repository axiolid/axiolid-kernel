//! Normative v0.4 downstream integration profiles.

use crate::{
    ApiVersion, BoundaryContract, IntegrationProfile, INTEGRATION_API_VERSION, MINIMUM_RUST_VERSION,
};

/// Compatibility promise for one supported way of consuming Axiolid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProfileContract {
    pub profile: IntegrationProfile,
    pub name: &'static str,
    pub transport: &'static str,
    pub capability_discovery: &'static str,
    pub compatibility: &'static str,
    pub api_version: ApiVersion,
    pub abi_version: Option<ApiVersion>,
    pub minimum_rust_version: Option<&'static str>,
    pub boundary: BoundaryContract,
}

/// Profiles supported by the v0.4 integration protocol.
///
/// These rows promise boundary behavior, not geometry operations. A concrete
/// build advertises operation availability through `IntegrationDescriptor`;
/// operation traits and conformance evidence remain authoritative.
pub const V04_PROFILE_CONTRACTS: [ProfileContract; 3] = [
    ProfileContract {
        profile: IntegrationProfile::RustLeaf,
        name: "rust-leaf",
        transport: "Rust packages selected directly",
        capability_discovery: "selected operation traits plus Backend::descriptor",
        compatibility: "Cargo SemVer per selected leaf package",
        api_version: INTEGRATION_API_VERSION,
        abi_version: None,
        minimum_rust_version: Some(MINIMUM_RUST_VERSION),
        boundary: BoundaryContract::rust(),
    },
    ProfileContract {
        profile: IntegrationProfile::RustFacade,
        name: "rust-facade",
        transport: "feature-gated axiolid package",
        capability_discovery: "compiled IntegrationDescriptor plus operation traits",
        compatibility: "Cargo SemVer and additive feature names",
        api_version: INTEGRATION_API_VERSION,
        abi_version: None,
        minimum_rust_version: Some(MINIMUM_RUST_VERSION),
        boundary: BoundaryContract::rust(),
    },
    ProfileContract {
        profile: IntegrationProfile::NativeC,
        name: "native-c",
        transport: "C ABI with opaque handles",
        capability_discovery: "ABI version and runtime IntegrationDescriptor query",
        compatibility: "ABI major match; additive minor and patch releases",
        api_version: INTEGRATION_API_VERSION,
        abi_version: Some(INTEGRATION_API_VERSION),
        minimum_rust_version: None,
        boundary: BoundaryContract::native(),
    },
];

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn profile_names_are_unique_and_every_boundary_is_explicit() {
        let names: BTreeSet<_> = V04_PROFILE_CONTRACTS
            .iter()
            .map(|profile| profile.name)
            .collect();
        assert_eq!(names.len(), V04_PROFILE_CONTRACTS.len());
        for profile in V04_PROFILE_CONTRACTS {
            assert!(profile.boundary.right_handed_cartesian_f64);
            assert!(profile.boundary.caller_defined_consistent_units);
            assert!(profile.boundary.explicit_tolerance);
        }
    }

    #[test]
    fn only_the_native_profile_promises_an_abi() {
        for profile in V04_PROFILE_CONTRACTS {
            assert_eq!(
                profile.abi_version.is_some(),
                profile.profile == IntegrationProfile::NativeC
            );
        }
    }
}
