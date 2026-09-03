use axiolid_contracts::{
    capability_ids, ApiVersion, BoundaryContract, CapabilityDescriptor, CapabilityRequirement,
    Exactness, IntegrationDescriptor, IntegrationProfile, Ownership, Representation,
    RequirementRefusal, ThreadSafety, INTEGRATION_API_VERSION, MINIMUM_RUST_VERSION,
};

fn mesh_boolean(exactness: Exactness) -> CapabilityDescriptor {
    CapabilityDescriptor {
        id: capability_ids::MESH_BOOLEAN,
        operation: axiolid_contracts::Operation::MeshBoolean,
        provider: axiolid_contracts::BackendId::new("test-provider"),
        required_feature: "mesh-boolean",
        inputs: &[Representation::TriangleMesh],
        output: Representation::TriangleMesh,
        exactness,
        deterministic: true,
    }
}

fn descriptor(capability: CapabilityDescriptor) -> IntegrationDescriptor {
    IntegrationDescriptor {
        api_version: INTEGRATION_API_VERSION,
        abi_version: None,
        profile: IntegrationProfile::RustFacade,
        minimum_rust_version: Some(MINIMUM_RUST_VERSION),
        enabled_features: vec!["mesh-boolean"],
        representations: vec![Representation::TriangleMesh],
        capabilities: vec![capability],
        boundary: BoundaryContract::rust(),
    }
}

#[test]
fn capability_handshake_accepts_only_the_advertised_fidelity() {
    let approximate = descriptor(mesh_boolean(Exactness::ToleranceBounded));
    let requirement = CapabilityRequirement {
        id: capability_ids::MESH_BOOLEAN,
        output: Representation::TriangleMesh,
        exactness: Exactness::Exact,
        deterministic: true,
    };

    assert_eq!(
        approximate.require(requirement),
        Err(RequirementRefusal::ExactnessUnavailable {
            capability: capability_ids::MESH_BOOLEAN,
            required: Exactness::Exact,
            advertised: Exactness::ToleranceBounded,
        })
    );

    let exact = descriptor(mesh_boolean(Exactness::Exact));
    assert!(exact.require(requirement).is_ok());
}

#[test]
fn absent_capability_is_a_typed_refusal() {
    let descriptor = IntegrationDescriptor::empty(IntegrationProfile::RustLeaf);
    let requirement = CapabilityRequirement {
        id: capability_ids::MESH_SECTION,
        output: Representation::Profile2d,
        exactness: Exactness::ToleranceBounded,
        deterministic: true,
    };

    assert_eq!(
        descriptor.require(requirement),
        Err(RequirementRefusal::CapabilityUnavailable {
            capability: capability_ids::MESH_SECTION,
        })
    );
}

#[test]
fn api_compatibility_is_major_versioned() {
    let descriptor = IntegrationDescriptor::empty(IntegrationProfile::RustLeaf);
    assert!(descriptor.supports_api(ApiVersion::new(0, 4, 0)));
    assert!(descriptor.supports_api(ApiVersion::new(0, 3, 0)));
    assert!(!descriptor.supports_api(ApiVersion::new(1, 0, 0)));
    assert!(matches!(
        descriptor.require_api(ApiVersion::new(1, 0, 0)),
        Err(RequirementRefusal::ApiVersionUnavailable { .. })
    ));
}

#[test]
fn boundary_contracts_make_ownership_and_concurrency_explicit() {
    let rust = BoundaryContract::rust();
    assert_eq!(rust.ownership, Ownership::RustValues);
    assert_eq!(rust.thread_safety, ThreadSafety::SendSyncValues);
    assert!(rust.explicit_tolerance);
    assert!(rust.caller_defined_consistent_units);

    let native = BoundaryContract::native();
    assert_eq!(native.ownership, Ownership::OpaqueOwnedHandles);
    assert_eq!(native.thread_safety, ThreadSafety::ContextSendNotSync);
}
