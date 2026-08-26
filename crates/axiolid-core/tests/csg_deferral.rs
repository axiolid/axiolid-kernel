//! Executable guard for ADR 0017: CSG stays deferred until its contract lands.
//!
//! A prose ADR does not fail a build. These tests do. They pin the gaps ADR
//! 0017 measured, so the deferral cannot lapse by accident and so each gap
//! announces itself the moment someone closes it.

use std::path::{Path, PathBuf};

fn crates_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/ dir")
        .to_path_buf()
}

fn read(relative: &str) -> String {
    let path = crates_dir().join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("read {path:?}: {error}"))
}

fn adr_0017() -> String {
    let path = crates_dir()
        .parent()
        .expect("repo root")
        .join("docs/adr/0017-solid-boolean-contract-before-implementation.md");
    std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("read {path:?}: {error}"))
}

/// The deferral is only meaningful while the ADR that defines it exists and is
/// registered. A deleted or unindexed ADR would make every other test here a
/// guard over nothing.
#[test]
fn the_deferral_decision_is_recorded_and_indexed() {
    let adr = adr_0017();
    for section in [
        "Public operation semantics",
        "Input / topology requirements",
        "Diagnostics",
        "Resource and cancellation contracts",
        "Scalar correctness oracle",
        "Provider conformance tests",
    ] {
        assert!(
            adr.contains(section),
            "ADR 0017 must define the `{section}` contract"
        );
    }

    let index = crates_dir()
        .parent()
        .expect("repo root")
        .join("docs/adr/README.md");
    let index = std::fs::read_to_string(index).expect("read ADR index");
    assert!(
        index.contains("0017-solid-boolean-contract-before-implementation.md"),
        "ADR 0017 must be listed in the ADR index"
    );
}

/// ADR 0017 requires the 3D operation set to match `axiolid-overlay`'s 2D set.
///
/// Today it does not: `BooleanOperator` mirrors the adopted backend's three
/// ops. This test documents the gap and FAILS THE MOMENT IT IS CLOSED, which
/// is the point -- closing it means the contract work started, and this guard
/// must then be replaced by real conformance tests rather than left stale.
#[test]
fn operation_set_gap_versus_overlay_is_still_open() {
    let core_ops = read("axiolid-core/src/operation.rs");
    let has_symmetric_difference =
        core_ops.contains("SymmetricDifference") || core_ops.contains("Xor");

    assert!(
        !has_symmetric_difference,
        "BooleanOperator gained symmetric difference: ADR 0017 section 1 is \
         being implemented. Replace this guard with the conformance suite from \
         section 6 and update ADR 0017's status."
    );

    let overlay = read("axiolid-overlay/src/lib.rs");
    assert!(
        overlay.contains("Xor"),
        "the 2D contract is the alignment target and must keep its fourth operation"
    );
}

/// ADR 0012 orders the scalar reference before any optimized provider. For
/// booleans that ordering was not honoured. This asserts the gap is still the
/// one ADR 0017 recorded -- and fires when the oracle appears.
#[test]
fn scalar_boolean_oracle_gap_is_still_open() {
    let scalar_src = crates_dir().join("axiolid-scalar/src");
    let mut implements_boolean = false;
    let mut stack = vec![scalar_src];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).expect("read axiolid-scalar/src") {
            let path = entry.expect("dir entry").path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                let text = std::fs::read_to_string(&path).expect("read scalar source");
                if text.contains("impl MeshBoolean") {
                    implements_boolean = true;
                }
            }
        }
    }

    assert!(
        !implements_boolean,
        "axiolid-scalar gained a MeshBoolean impl: the ADR 0017 section 5 \
         oracle exists. Wire the conformance suite to it and retire this guard."
    );
}

/// A boolean provider must not be registrable without passing a shared
/// conformance suite (ADR 0017 section 6). No such suite exists yet, and the
/// current provider tests bind to the concrete type.
#[test]
fn provider_conformance_suite_gap_is_still_open() {
    let kernel_src = crates_dir().join("axiolid-kernel/src");
    let mut has_suite = false;
    for entry in std::fs::read_dir(&kernel_src).expect("read axiolid-kernel/src") {
        let path = entry.expect("dir entry").path();
        if path.extension().is_some_and(|ext| ext == "rs") {
            let text = std::fs::read_to_string(&path).expect("read kernel source");
            if text.contains("pub fn assert_mesh_boolean_conformance")
                || text.contains("pub mod conformance")
            {
                has_suite = true;
            }
        }
    }

    assert!(
        !has_suite,
        "a MeshBoolean conformance harness appeared: ADR 0017 section 6 is \
         landing. Make it mandatory for registration and retire this guard."
    );
}

/// `GeomError::Cancelled` is declared but produced nowhere, so the cancellation
/// contract is currently fiction. ADR 0017 section 4 makes it real.
#[test]
fn cancellation_contract_gap_is_still_open() {
    let error = read("axiolid-kernel/src/error.rs");
    assert!(
        error.contains("Cancelled"),
        "the cancellation variant is the contract placeholder ADR 0017 builds on"
    );

    let execution = read("axiolid-kernel/src/execution.rs");
    let has_token =
        execution.contains("CancellationToken") || execution.contains("fn is_cancelled");
    assert!(
        !has_token,
        "ExecutionOptions gained cancellation: ADR 0017 section 4 is landing. \
         Add the conformance test that proves providers actually poll it."
    );
}

/// Preconditions live in the L3 adapter today. ADR 0017 section 2 moves them
/// into the L2 contract so every provider sees identical admissible input.
#[test]
fn precondition_ownership_gap_is_still_open() {
    let convert = read("axiolid-boolmesh/src/convert.rs");
    assert!(
        convert.contains("fn to_manifold"),
        "the adapter-side validation this ADR relocates must still be findable"
    );

    let kernel_boolean = read("axiolid-kernel/src/boolean.rs");
    let validates_in_l2 =
        kernel_boolean.contains("SolidValidation") || kernel_boolean.contains("fn validate_solid");
    assert!(
        !validates_in_l2,
        "the kernel gained solid validation: ADR 0017 section 2 is landing. \
         Assert precondition parity across providers and retire this guard."
    );
}

/// The boolean returns a bare mesh while overlay and field both return
/// structured evidence. ADR 0017 section 3 closes that inconsistency.
#[test]
fn boolean_evidence_gap_is_still_open() {
    let kernel_boolean = read("axiolid-kernel/src/boolean.rs");
    let has_evidence =
        kernel_boolean.contains("BooleanEvidence") || kernel_boolean.contains("BooleanOutcome");
    assert!(
        !has_evidence,
        "MeshBoolean gained structured evidence: ADR 0017 section 3 is landing."
    );

    // The two contracts it must eventually match already report evidence.
    assert!(read("axiolid-overlay/src/lib.rs").contains("OverlayEvidence"));
    assert!(read("axiolid-field/src/field.rs").contains("FieldEvidence"));
}

/// ADR 0011 keeps native accelerator backends out of tree; ADR 0017 depends on
/// that to stop a C++ kernel defining these semantics by arriving first.
#[test]
fn no_native_csg_backend_has_been_introduced() {
    const NATIVE_MARKERS: &[&str] = &["manifold3d", "opencascade", "occt", "cgal", "carve", "cork"];

    for entry in std::fs::read_dir(crates_dir()).expect("read crates/") {
        let path = entry.expect("dir entry").path();
        let manifest = path.join("Cargo.toml");
        if !manifest.is_file() {
            continue;
        }
        let text = std::fs::read_to_string(&manifest)
            .expect("read manifest")
            .to_lowercase();
        for marker in NATIVE_MARKERS {
            assert!(
                !text.contains(marker),
                "{}: native CSG dependency `{marker}` would let a C++ kernel \
                 define Axiolid's boolean semantics (ADR 0011, ADR 0017)",
                path.display()
            );
        }
    }
}
