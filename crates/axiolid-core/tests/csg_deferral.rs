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
