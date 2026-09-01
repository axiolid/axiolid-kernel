use axiolid_core::Tolerance;
use axiolid_generate::{GenerationOutput, GenerationRequest, TessellationRequest};

#[test]
fn exact_and_tessellation_are_distinct_explicit_contracts() {
    let exact = GenerationRequest::ExactBRep;
    assert!(matches!(exact, GenerationRequest::ExactBRep));

    let requested = TessellationRequest::new(Tolerance::METRE);
    assert_eq!(requested.tolerance(), Tolerance::METRE);
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
