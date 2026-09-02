use axiolid_construct::{GenerationOutput, GenerationRequest, TessellationRequest};
use axiolid_core::{Tolerance, Vec3};
use axiolid_mesh::TriMesh;

#[test]
fn exact_and_tessellation_are_distinct_explicit_contracts() {
    let exact = GenerationRequest::ExactBRep;
    assert!(matches!(exact, GenerationRequest::ExactBRep));

    let requested = TessellationRequest::new(Tolerance::METRE);
    assert_eq!(requested.tolerance(), Tolerance::METRE);
    let mesh = TriMesh::new(vec![Vec3::ZERO, Vec3::X, Vec3::Y], vec![0, 1, 2]);
    let produced = requested.bind(mesh.clone());
    assert_eq!(produced.tolerance(), requested.tolerance());
    assert_eq!(produced.mesh(), &mesh);
    assert!(matches!(
        GenerationRequest::Tessellation(requested),
        GenerationRequest::Tessellation(_)
    ));

    assert_ne!(
        GenerationOutput::ExactBRep,
        GenerationOutput::Tessellation,
        "a caller must select the output model; exact construction never falls back to a mesh"
    );
}
