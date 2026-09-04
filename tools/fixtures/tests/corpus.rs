//! The corpus must describe itself honestly (#18).

use axiolid_fixtures::{coplanar_contact, corpus};

/// Every fixture states its source, licence and expectation.
///
/// An unattributed fixture is a redistribution risk and an unexplained
/// failure later, so the corpus refuses to carry one.
#[test]
fn every_fixture_carries_complete_provenance() {
    for fixture in corpus() {
        let p = fixture.provenance;
        assert!(!p.source.is_empty(), "{}: no source", fixture.name);
        assert!(!p.licence.is_empty(), "{}: no licence", fixture.name);
        assert!(
            !p.expectation.is_empty(),
            "{}: no expectation",
            fixture.name
        );
        assert!(
            p.source.len() > 20,
            "{}: source too vague to check",
            fixture.name
        );
    }
}

/// Fixture names are unique, so a failure message identifies one case.
#[test]
fn fixture_names_are_unique() {
    let mut names: Vec<&str> = corpus().iter().map(|f| f.name).collect();
    names.sort_unstable();
    let count = names.len();
    names.dedup();
    assert_eq!(count, names.len(), "duplicate fixture name");
}

/// The corpus is non-trivial and the paired fixtures are distinct.
#[test]
fn the_corpus_is_populated() {
    assert!(corpus().len() >= 6, "corpus is too thin to be useful");
    let (left, right) = coplanar_contact();
    assert_ne!(left.mesh, right.mesh, "the pair must differ");
}
