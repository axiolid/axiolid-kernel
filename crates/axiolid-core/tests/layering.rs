//! Architecture test: the geometry stack is layered, and the layering is real.
//!
//! `packages/ifc/ifc-geometry/tests/no_backend_dependency.rs` guards the
//! IFC-to-geometry seam from the IFC side. It says nothing about the geometry
//! side, so until this file existed the internal layering of
//! `crates/` was documented in `AGENTS.md` and enforced by nobody.
//!
//! Two invariants live here.
//!
//! # 1. Geometry never depends on IFC (the direction of the seam)
//!
//! The whole premise is that the geometry stack is format-agnostic and could be
//! consumed by a STEP-CAD or CityGML front end. One `ifc-model` dependency in
//! any crate under `crates/` silently ends that, and it would end it
//! without breaking a build, because the workspace makes the crate reachable.
//!
//! # 2. Data flows up the tiers, never down
//!
//! ```text
//!   L0  math / data      axiolid-core                      (no siblings at all)
//!    ^
//!   L1  representation   axiolid-mesh, axiolid-profile, axiolid-curve,
//!                        axiolid-surface, axiolid-topology, axiolid-primitive,
//!                        axiolid-model
//!    ^
//!   L2  algorithms       axiolid-sweep, axiolid-tessellate, axiolid-spatial,
//!                        axiolid-measure, axiolid-overlay, axiolid-heal, axiolid-kernel
//!    ^
//!   L3  implementations  axiolid-backend-cpu, axiolid-backend-gpu
//!    ^
//!   L4  facade           axiolid
//! ```
//!
//! A crate may depend on a lower tier or on its own tier (`axiolid-surface` needs
//! `axiolid-curve` for trimming; both are representation). It may never depend on
//! a higher one. The rule that matters most is the L1 one: representation types
//! must stay usable without dragging in an algorithm crate, because that is what
//! lets a foreign kernel accept our `TriMesh` without accepting our kernel.
//!
//! `axiolid-core` is special-cased to zero geometry dependencies. It is the shared
//! vocabulary; the moment it depends on a sibling, the siblings stop being
//! siblings and the backends stop being swappable for one another.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// Tier of each crate under `crates/`, low to high.
///
/// This list is the specification. A crate missing from it fails
/// [`every_geometry_crate_has_a_declared_tier`], which forces the question
/// "which layer is this?" to be answered when the crate is created rather than
/// inferred by whoever reads the dependency graph later.
const TIERS: &[(&str, u8)] = &[
    // L0 -- data and math only. No algorithms, no backends.
    ("axiolid-core", 0),
    // L1 -- geometric representation. Types, evaluation, no solving.
    ("axiolid-mesh", 1),
    ("axiolid-profile", 1),
    ("axiolid-curve", 1),
    ("axiolid-surface", 1),
    ("axiolid-topology", 1),
    ("axiolid-primitive", 1),
    // `axiolid-model` is the format-neutral item graph every front end lowers into
    // and every kernel consumes. It sits at the top of L1 because it is a
    // representation that composes the other representations -- it solves
    // nothing itself.
    ("axiolid-model", 1),
    // L2 -- algorithms over those representations, plus the backend contract.
    ("axiolid-sweep", 2),
    ("axiolid-tessellate", 2),
    ("axiolid-spatial", 2),
    ("axiolid-measure", 2),
    ("axiolid-overlay", 2),
    // Sampled layered fields: coverage, morphology, clearance, and opt-in
    // geometry-only traversal. Algorithms over representations, so L2.
    ("axiolid-field", 2),
    ("axiolid-heal", 2),
    ("axiolid-kernel", 2),
    // The scalar reference implementation (ADR 0012). Consumes the kernel
    // contracts to report certified results; owns algorithms, not scheduling.
    ("axiolid-scalar", 2),
    // L3 -- execution contexts and operation adapters.
    // L3 -- concrete implementations. `axiolid-boolmesh` adapts an adopted
    // upstream crate (ADR 0014); it is an implementation, not a contract.
    ("axiolid-boolmesh", 3),
    ("axiolid-compile", 3),
    ("axiolid-backend-cpu", 3),
    ("axiolid-backend-gpu", 3),
    // L4 -- opt-in facade over lower layers.
    ("axiolid", 4),
];

#[test]
fn operation_traits_are_the_only_capability_claim() {
    let geometry = geometry_dir();
    let metadata = std::fs::read_to_string(geometry.join("axiolid-kernel/src/capability.rs"))
        .expect("read capability metadata");
    assert!(
        !metadata.contains("OperationSupport"),
        "operation support metadata duplicates Rust trait implementations"
    );

    let cpu = std::fs::read_to_string(geometry.join("axiolid-backend-cpu/src/execution.rs"))
        .expect("read CPU context");
    assert!(
        !cpu.contains("impl MeshBoolean for CpuExecution"),
        "an execution context must not implement an unavailable operation"
    );
}

#[test]
fn every_geometry_crate_has_an_explicit_unsafe_policy() {
    for crate_name in geometry_crates() {
        let root = std::fs::read_to_string(geometry_dir().join(&crate_name).join("src/lib.rs"))
            .expect("read crate root");
        if crate_name == "axiolid-backend-cpu" {
            assert!(root.contains("#![deny(unsafe_op_in_unsafe_fn)]"));
        } else {
            assert!(
                root.contains("#![forbid(unsafe_code)]"),
                "{crate_name} must forbid unsafe code"
            );
        }
    }
}

#[test]
fn geometry_crates_do_not_declare_native_cpp_bridges() {
    const FORBIDDEN: &[&str] = &[
        "bindgen",
        "cmake",
        "cxx",
        "cxx-build",
        "manifold3d",
        "opencascade",
    ];
    for crate_name in geometry_crates() {
        for dependency in declared_dependencies(&manifest_of(&crate_name)) {
            assert!(
                !FORBIDDEN.contains(&dependency.as_str()),
                "{crate_name} directly depends on forbidden native bridge {dependency}"
            );
        }
    }
}

fn geometry_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

/// Crate names from the IFC ecosystem that a format-agnostic kernel must never
/// depend on. This list is intentionally owned here: Axiolid is standalone and
/// must not reach into a sibling repository merely to prove its own boundary.
const FORBIDDEN_FORMAT_CRATES: &[&str] = &[
    "ifc",
    "ifc-alignment",
    "ifc-geometry",
    "ifc-georef",
    "ifc-model",
    "ifc-schema",
    "ifc-step",
    "ifc-xml",
];

/// Crate directory names actually present under `crates/`.
fn geometry_crates() -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    for entry in std::fs::read_dir(geometry_dir()).expect("crates must exist") {
        let path = entry.unwrap().path();
        if path.join("Cargo.toml").exists() {
            found.insert(path.file_name().unwrap().to_string_lossy().to_string());
        }
    }
    found
}

/// Every dependency name declared in a manifest, from any dependency table.
///
/// `dev-dependencies` count deliberately: a test that reaches up a tier proves
/// the layering is not real in practice, whatever the release graph says.
/// Comments are stripped so a `# never depend on ifc-model` note is not a hit.
fn declared_dependencies(manifest: &str) -> BTreeSet<String> {
    let mut deps = BTreeSet::new();
    let mut in_deps = false;
    for raw in manifest.lines() {
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.starts_with('[') {
            // `[dependencies]`, `[dev-dependencies]`, `[target.'..'.dependencies]`
            in_deps = line.trim_end_matches(']').ends_with("dependencies");
            continue;
        }
        if !in_deps || line.is_empty() {
            continue;
        }
        // `axiolid-mesh.workspace = true` and `axiolid-kernel = { .. }` both start
        // with the crate name; take the token before `=`, `.` or whitespace.
        let name: String = line
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
            .collect();
        if !name.is_empty() {
            deps.insert(name);
        }
    }
    deps
}

fn manifest_of(crate_name: &str) -> String {
    let path = geometry_dir().join(crate_name).join("Cargo.toml");
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn tier_of(crate_name: &str) -> Option<u8> {
    TIERS
        .iter()
        .find(|(n, _)| *n == crate_name)
        .map(|(_, t)| *t)
}

/// The tier table must describe the directory, not a remembered version of it.
/// Without this, adding a crate would silently opt it out of every rule below.
#[test]
fn every_geometry_crate_has_a_declared_tier() {
    let on_disk = geometry_crates();
    let declared: BTreeSet<String> = TIERS.iter().map(|(n, _)| n.to_string()).collect();

    let untiered: Vec<&String> = on_disk.difference(&declared).collect();
    assert!(
        untiered.is_empty(),
        "crates crates with no tier in TIERS: {untiered:?}\n\
         Add each to TIERS in this file with the layer it belongs to:\n\
         \t0 = math/data, 1 = representation, 2 = algorithms/contract,\n\
         \t3 = backend implementation, 4 = facade.\n\
         A crate outside the table is exempt from the layering rules, which is \
         how a layered design quietly stops being one."
    );

    let missing: Vec<&String> = declared.difference(&on_disk).collect();
    assert!(
        missing.is_empty(),
        "TIERS names crates that do not exist: {missing:?}\n\
         A stale entry lets a renamed crate escape the layering checks."
    );
}

/// Invariant 1: Axiolid has no dependency on a source-format layer.
#[test]
fn geometry_never_depends_on_source_formats() {
    for name in geometry_crates() {
        for dep in declared_dependencies(&manifest_of(&name)) {
            assert!(
                !FORBIDDEN_FORMAT_CRATES.contains(&dep.as_str()),
                "crates/{name} depends on `{dep}`, a source-format crate.\n\
                 The geometry stack is format-agnostic on purpose: it is what a \
                 STEP-CAD or CityGML front end would reuse. Source-format semantics \
                 belong in that format's adapter, which translates INTO Axiolid types."
            );
        }
    }
}

/// Invariant 2: dependencies point down the stack or sideways, never up.
#[test]
fn geometry_dependencies_never_point_up_a_tier() {
    for name in geometry_crates() {
        let Some(tier) = tier_of(&name) else { continue };
        for dep in declared_dependencies(&manifest_of(&name)) {
            let Some(dep_tier) = tier_of(&dep) else {
                continue;
            };
            assert!(
                dep_tier <= tier,
                "crates/{name} (tier {tier}) depends on `{dep}` \
                 (tier {dep_tier}).\n\
                 Geometry is layered math -> representation -> algorithms. A \
                 representation crate that depends on an algorithm crate forces \
                 every consumer of the TYPES to compile the SOLVERS, which is \
                 exactly the coupling that makes a kernel unswappable."
            );
        }
    }
}

/// `axiolid-core` is the shared vocabulary. It gets no geometry dependencies at
/// all, not even same-tier ones, because there is no lower tier to escape to.
#[test]
fn axiolid_core_has_no_geometry_dependencies() {
    for dep in declared_dependencies(&manifest_of("axiolid-core")) {
        assert!(
            !dep.starts_with("axiolid-"),
            "axiolid-core depends on `{dep}`.\n\
             axiolid-core is the root: every other crate and every backend names \
             its types. If it depends on a sibling, that sibling is dragged into \
             everything and the backends stop being true alternatives."
        );
    }
}

/// Dependency checks cannot catch format vocabulary hidden in source names.
#[test]
fn geometry_sources_are_format_agnostic() {
    fn collect_rs(dir: &Path, out: &mut Vec<PathBuf>) {
        for entry in std::fs::read_dir(dir).expect("read geometry source directory") {
            let path = entry.expect("read geometry source entry").path();
            if path.is_dir() {
                collect_rs(&path, out);
            } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
                out.push(path);
            }
        }
    }

    for crate_name in geometry_crates() {
        let mut files = Vec::new();
        collect_rs(&geometry_dir().join(crate_name).join("src"), &mut files);
        for file in files {
            let source = std::fs::read_to_string(&file).expect("read geometry source");
            assert!(
                !source.to_ascii_lowercase().contains("ifc"),
                "{} contains IFC vocabulary; source-format semantics belong in an adapter",
                file.display()
            );
        }
    }
}

#[test]
fn geometry_sources_have_no_panicking_placeholders() {
    fn collect_rs(dir: &Path, out: &mut Vec<PathBuf>) {
        for entry in std::fs::read_dir(dir).expect("read geometry source directory") {
            let path = entry.expect("read geometry source entry").path();
            if path.is_dir() {
                collect_rs(&path, out);
            } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
                out.push(path);
            }
        }
    }

    let forbidden = [concat!("to", "do!("), concat!("unimplemented", "!(")];
    for crate_name in geometry_crates() {
        let mut files = Vec::new();
        collect_rs(&geometry_dir().join(crate_name).join("src"), &mut files);
        for file in files {
            let source = std::fs::read_to_string(&file).expect("read geometry source");
            for &placeholder in &forbidden {
                assert!(
                    !source.contains(placeholder),
                    "{} contains panicking scaffold placeholder {placeholder}; expose a contract or return structured Unsupported instead",
                    file.display()
                );
            }
        }
    }
}

/// Scaffold files must participate in compilation; orphan `.rs` files rot.
#[test]
fn every_geometry_source_module_is_declared() {
    fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
        for entry in std::fs::read_dir(dir).expect("read source directory") {
            let path = entry.expect("read source entry").path();
            if path.is_dir() {
                collect(&path, out);
            } else if path.extension().and_then(|v| v.to_str()) == Some("rs") {
                out.push(path);
            }
        }
    }

    for crate_name in geometry_crates() {
        let src = geometry_dir().join(crate_name).join("src");
        let mut files = Vec::new();
        collect(&src, &mut files);
        for file in files {
            if file.file_name().and_then(|v| v.to_str()) == Some("lib.rs") {
                continue;
            }
            let stem = file
                .file_stem()
                .and_then(|v| v.to_str())
                .expect("Rust file stem");
            let parent = file.parent().expect("module parent");
            let candidate = if parent == src {
                src.join("lib.rs")
            } else {
                parent.with_extension("rs")
            };
            let declaration = if candidate.exists() {
                candidate
            } else {
                parent.join("mod.rs")
            };
            let source =
                std::fs::read_to_string(&declaration).expect("read module declaration file");
            assert!(
                source.contains(&format!("mod {stem};")),
                "{} is not declared by {}",
                file.display(),
                declaration.display()
            );
        }
    }
}
