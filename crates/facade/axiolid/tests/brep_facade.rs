#![cfg(feature = "generate")]

#[test]
fn facade_exposes_exact_brep_contract_alongside_generation_requests() {
    let _builder = axiolid::brep::ExactBRepBuilder::default();
    let request = axiolid::generate::GenerationRequest::ExactBRep;
    assert!(request.requires_exact_brep());
}
