//! Executable guard: the field API must not name a domain verdict.
//!
//! Axiolid may report `route exists`, `no route under this envelope`, and
//! `clearance = X`. It must never claim accessibility, ADA compliance,
//! wheelchair suitability, a valid escape route, or a vendor rule violation.
//! Those are application judgements built on top of this geometry.
//!
//! This is a source-level gate rather than a type-level one because the risk
//! is vocabulary creep during future edits, not a compile error today.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

/// Substrings that would signal domain policy leaking into the kernel.
const FORBIDDEN: &[&str] = &[
    "accessib",
    "wheelchair",
    "ada_",
    "ada-",
    "escape",
    "egress",
    "evacuat",
    "compliant",
    "compliance",
    "violation",
    "solibri",
    "ifc",
    "building_code",
    "barrier_free",
    "handicap",
    "walkab",
    "mobility_impair",
];

/// Terms permitted in prose that explicitly *disclaims* a domain meaning.
///
/// The guard scans code identifiers and doc text alike, so a doc line that
/// says "this is not an accessibility verdict" would otherwise trip it. Such
/// lines must carry this marker so the disclaimer stays greppable and honest.
const DISCLAIMER: &str = "NOT-A-VERDICT";

fn source_files(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    for entry in fs::read_dir(dir).expect("field src dir is readable") {
        let path = entry.expect("readable dir entry").path();
        if path.is_dir() {
            source_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

#[test]
fn the_field_api_never_names_a_domain_verdict() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    source_files(&root, &mut files);
    assert!(!files.is_empty(), "guard must actually scan sources");

    let mut offences = BTreeSet::new();
    for path in &files {
        let text = fs::read_to_string(path).expect("source is valid utf-8");
        for (number, line) in text.lines().enumerate() {
            if line.contains(DISCLAIMER) {
                continue;
            }
            let lowered = line.to_ascii_lowercase();
            for term in FORBIDDEN {
                if lowered.contains(term) {
                    offences.insert(format!(
                        "{}:{}: forbidden term {:?}",
                        path.file_name().expect("named file").to_string_lossy(),
                        number + 1,
                        term
                    ));
                }
            }
        }
    }

    assert!(
        offences.is_empty(),
        "domain vocabulary leaked into the neutral field API:\n{}",
        offences.into_iter().collect::<Vec<_>>().join("\n")
    );
}

#[test]
fn the_guard_can_actually_fail() {
    // A guard that cannot fail is decorative. Prove the detector fires on a
    // line that would be a real violation if it appeared in the sources.
    let sample = "pub fn is_wheelchair_accessible() -> bool { true }";
    let lowered = sample.to_ascii_lowercase();
    assert!(
        FORBIDDEN.iter().any(|term| lowered.contains(term)),
        "detector failed to flag an obvious domain verdict"
    );
}
