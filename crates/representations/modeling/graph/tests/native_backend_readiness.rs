//! Executable gate: the geometry data plane stays transferable to a native
//! accelerator backend.
//!
//! Why this exists: `axiolid-backend-gpu` is an API-neutral seam, and the roadmap
//! expects a native CUDA/HIP executor to arrive later as an out-of-tree crate.
//! Such a crate must be able to walk a `GeometryGraph` and copy its payloads
//! across an FFI boundary. That is only possible while node payloads stay
//! *owned plain data*: no trait objects, no closures, no borrowed references,
//! no shared-ownership handles, no interior mutability.
//!
//! Today the representation satisfies this by good taste rather than by any
//! rule. One future `Box<dyn Evaluator>` field would silently destroy native
//! backend viability, and the cost would only surface years later when the
//! bridge is written. This gate turns the expectation into a build failure.
//!
//! This is a *structural* gate over the public representation types. It does
//! not claim the types are `#[repr(C)]` or directly memcpy-able; it claims they
//! stay mechanically walkable and copyable by a bridge crate.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use syn::visit::{self, Visit};

/// Crates whose public types make up the format-neutral data plane that a
/// native backend must be able to consume.
const DATA_PLANE_CRATES: &[(&str, &str)] = &[
    ("axiolid-core", "crates/foundation/core"),
    ("axiolid-mesh", "crates/representations/discrete/mesh"),
    ("axiolid-model", "crates/representations/modeling/graph"),
    ("axiolid-curve", "crates/representations/analytic/curve"),
    ("axiolid-surface", "crates/representations/analytic/surface"),
    ("axiolid-profile", "crates/representations/region/profile"),
    ("axiolid-primitive", "crates/representations/analytic/primitive"),
    ("axiolid-topology", "crates/representations/topology"),
];

/// Constructs that make a payload impossible to hand to a native backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Hostility {
    /// A trait object has no fixed layout and no way to cross FFI.
    TraitObject,
    /// A closure/function value cannot be transferred to a device.
    Callable,
    /// A borrowed reference makes ownership ambiguous across the boundary.
    Borrow,
    /// Shared/atomic ownership cannot be reconstructed on the far side.
    SharedOwnership,
    /// Interior mutability breaks the immutable-DAG contract a backend relies
    /// on when it uploads a graph once and compiles many roots.
    InteriorMutability,
    /// A raw pointer in a representation payload is unowned and unverifiable.
    RawPointer,
}

impl Hostility {
    fn reason(self) -> &'static str {
        match self {
            Self::TraitObject => "trait object (no fixed layout across FFI)",
            Self::Callable => "callable value (cannot be transferred to a device)",
            Self::Borrow => "borrowed reference (ambiguous ownership across FFI)",
            Self::SharedOwnership => "shared-ownership handle (not reconstructible across FFI)",
            Self::InteriorMutability => "interior mutability (breaks the immutable-DAG contract)",
            Self::RawPointer => "raw pointer (unowned, unverifiable payload)",
        }
    }
}

/// Type-name heads that indicate a hostile payload.
fn hostility_for_type_name(name: &str) -> Option<Hostility> {
    match name {
        "Rc" | "Arc" => Some(Hostility::SharedOwnership),
        "Cell" | "RefCell" | "UnsafeCell" | "Mutex" | "RwLock" | "OnceCell" | "OnceLock" => {
            Some(Hostility::InteriorMutability)
        }
        _ => None,
    }
}

fn workspace_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .ancestors()
        .find(|candidate| {
            std::fs::read_to_string(candidate.join("Cargo.toml"))
                .is_ok_and(|text| text.contains("[workspace]"))
        })
        .expect("workspace Cargo.toml above package")
        .to_path_buf()
}

fn collect_rust_sources(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_rust_sources(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

/// Walks public type definitions looking for FFI-hostile field types.
struct DataPlaneVisitor {
    /// `true` while inside a `#[cfg(test)]` item; test scaffolding is exempt.
    in_test: bool,
    findings: Vec<String>,
}

fn is_test_gated(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| {
        if !attr.path().is_ident("cfg") {
            return false;
        }
        let mut found = false;
        let _ = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("test") {
                found = true;
            }
            Ok(())
        });
        found
    })
}

fn is_public(vis: &syn::Visibility) -> bool {
    matches!(vis, syn::Visibility::Public(_))
}

/// Error types are Rust-side diagnostics, not device payloads.
///
/// A bridge crate converts an error into a code plus a message at the
/// boundary; it never uploads one. `&'static str` labels inside an error are
/// therefore legitimate, so error types are outside this rule's scope.
fn is_diagnostic_type(name: &str) -> bool {
    name.ends_with("Error")
}

impl DataPlaneVisitor {
    fn inspect_type(&mut self, owner: &str, field: &str, ty: &syn::Type) {
        let mut hits = BTreeSet::new();
        scan_type(ty, &mut hits);
        for hostility in hits {
            self.findings.push(format!(
                "{owner}.{field}: {reason}",
                reason = hostility.reason()
            ));
        }
    }

    fn inspect_fields(&mut self, owner: &str, fields: &syn::Fields) {
        for (index, field) in fields.iter().enumerate() {
            let name = field
                .ident
                .as_ref()
                .map_or_else(|| index.to_string(), ToString::to_string);
            self.inspect_type(owner, &name, &field.ty);
        }
    }
}

/// Recursively scan a type for hostile constructs.
fn scan_type(ty: &syn::Type, hits: &mut BTreeSet<Hostility>) {
    match ty {
        syn::Type::TraitObject(_) | syn::Type::ImplTrait(_) => {
            hits.insert(Hostility::TraitObject);
        }
        syn::Type::Reference(reference) => {
            hits.insert(Hostility::Borrow);
            scan_type(&reference.elem, hits);
        }
        syn::Type::Ptr(pointer) => {
            hits.insert(Hostility::RawPointer);
            scan_type(&pointer.elem, hits);
        }
        syn::Type::BareFn(_) => {
            hits.insert(Hostility::Callable);
        }
        syn::Type::Array(array) => scan_type(&array.elem, hits),
        syn::Type::Slice(slice) => scan_type(&slice.elem, hits),
        syn::Type::Paren(inner) => scan_type(&inner.elem, hits),
        syn::Type::Group(group) => scan_type(&group.elem, hits),
        syn::Type::Tuple(tuple) => {
            for element in &tuple.elems {
                scan_type(element, hits);
            }
        }
        syn::Type::Path(path) => {
            for segment in &path.path.segments {
                let name = segment.ident.to_string();
                if let Some(hostility) = hostility_for_type_name(&name) {
                    hits.insert(hostility);
                }
                // `Fn`/`FnMut`/`FnOnce` appear as path segments in bounds.
                if matches!(name.as_str(), "Fn" | "FnMut" | "FnOnce") {
                    hits.insert(Hostility::Callable);
                }
                if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                    for arg in &args.args {
                        if let syn::GenericArgument::Type(inner) = arg {
                            scan_type(inner, hits);
                        }
                    }
                }
            }
        }
        _ => {}
    }
}

impl<'ast> Visit<'ast> for DataPlaneVisitor {
    fn visit_item_mod(&mut self, item: &'ast syn::ItemMod) {
        let was_test = self.in_test;
        self.in_test = was_test || is_test_gated(&item.attrs);
        visit::visit_item_mod(self, item);
        self.in_test = was_test;
    }

    fn visit_item_struct(&mut self, item: &'ast syn::ItemStruct) {
        let owner = item.ident.to_string();
        if !self.in_test
            && !is_test_gated(&item.attrs)
            && is_public(&item.vis)
            && !is_diagnostic_type(&owner)
        {
            self.inspect_fields(&owner, &item.fields);
        }
        visit::visit_item_struct(self, item);
    }

    fn visit_item_enum(&mut self, item: &'ast syn::ItemEnum) {
        if !self.in_test
            && !is_test_gated(&item.attrs)
            && is_public(&item.vis)
            && !is_diagnostic_type(&item.ident.to_string())
        {
            for variant in &item.variants {
                let owner = format!("{}::{}", item.ident, variant.ident);
                self.inspect_fields(&owner, &variant.fields);
            }
        }
        visit::visit_item_enum(self, item);
    }
}

fn hostile_payloads(source: &str) -> Vec<String> {
    let syntax = syn::parse_file(source).expect("data-plane source must parse");
    let mut visitor = DataPlaneVisitor {
        in_test: false,
        findings: Vec::new(),
    };
    visitor.visit_file(&syntax);
    visitor.findings
}

#[test]
fn data_plane_payloads_stay_transferable_to_a_native_backend() {
    let root = workspace_root();
    let mut violations = Vec::new();
    let mut inspected = 0_usize;

    for (crate_name, relative_path) in DATA_PLANE_CRATES {
        let src = root.join(relative_path).join("src");
        assert!(
            src.is_dir(),
            "data-plane crate {crate_name} must exist at {}",
            src.display()
        );
        let mut files = Vec::new();
        collect_rust_sources(&src, &mut files);
        assert!(!files.is_empty(), "{crate_name} has no Rust sources");
        for file in files {
            let text = std::fs::read_to_string(&file).expect("source readable");
            inspected += 1;
            for finding in hostile_payloads(&text) {
                violations.push(format!("{}: {finding}", file.display()));
            }
        }
    }

    assert!(inspected >= 20, "expected a real corpus, saw {inspected}");
    assert!(
        violations.is_empty(),
        "geometry data-plane payloads must stay transferable to a native \
         (CUDA/HIP) backend. A bridge crate has to walk and copy these values \
         across FFI; the constructs below make that impossible:\n{}",
        violations.join("\n")
    );
}

#[test]
fn the_gate_detects_each_hostile_construct() {
    let cases = [
        (
            "pub struct A { pub f: Box<dyn Fn(u32) -> u32> }",
            "trait object",
        ),
        ("pub struct B { pub f: fn(u32) -> u32 }", "callable"),
        ("pub struct C<'a> { pub f: &'a [u8] }", "borrowed"),
        (
            "pub struct D { pub f: std::rc::Rc<u32> }",
            "shared-ownership",
        ),
        (
            "pub struct E { pub f: std::sync::Arc<u32> }",
            "shared-ownership",
        ),
        (
            "pub struct F { pub f: core::cell::RefCell<u32> }",
            "interior",
        ),
        ("pub struct G { pub f: *const u8 }", "raw pointer"),
        (
            "pub enum H { V { f: Box<dyn Send> } }",
            "enum struct variant",
        ),
        (
            "pub enum I { V(std::sync::Mutex<u32>) }",
            "enum tuple variant",
        ),
        (
            "pub struct J { pub f: Vec<std::rc::Rc<u32>> }",
            "nested in Vec",
        ),
        (
            "pub struct K { pub f: Option<*mut u8> }",
            "nested in Option",
        ),
        (
            "pub struct L { pub f: (u32, std::sync::Arc<u8>) }",
            "in tuple",
        ),
        ("pub struct M { pub f: [std::rc::Rc<u8>; 4] }", "in array"),
    ];
    for (source, label) in cases {
        assert!(
            !hostile_payloads(source).is_empty(),
            "gate missed a hostile payload ({label}): {source}"
        );
    }
}

#[test]
fn the_gate_accepts_ordinary_owned_payloads() {
    let cases = [
        "pub struct A { pub positions: Vec<f64>, pub indices: Vec<u32> }",
        "pub struct B { pub id: u32, pub label: String }",
        "pub enum C { V(f64), W { a: u32, b: [f32; 3] } }",
        "pub struct D { pub nested: Option<Vec<(u32, f64)>> }",
        // Private types are out of scope: they are not part of the payload a
        // bridge crate observes.
        "struct Private { field: std::rc::Rc<u32> }",
    ];
    for source in cases {
        assert!(
            hostile_payloads(source).is_empty(),
            "gate produced a false positive for ordinary owned data: {source}\n{:?}",
            hostile_payloads(source)
        );
    }
}

#[test]
fn test_scaffolding_is_exempt_from_the_data_plane_rule() {
    // Test fixtures legitimately use `Arc`/`dyn`; the rule is about the
    // representation a backend consumes, not about test helpers.
    let source = "#[cfg(test)] mod tests { pub struct Fake { pub f: std::sync::Arc<u32> } }";
    assert!(hostile_payloads(source).is_empty());
}

/// The diagnostic exemption must stay narrow: only `*Error` types are excused,
/// and only because a bridge converts them to a code plus a message rather than
/// uploading them. A payload type must never inherit that excuse.
#[test]
fn only_error_types_receive_the_diagnostic_exemption() {
    assert!(
        hostile_payloads("pub enum GraphError { V { label: &'static str } }").is_empty(),
        "error types are diagnostics, not device payloads"
    );
    assert!(
        !hostile_payloads("pub enum GraphNode { V { label: &'static str } }").is_empty(),
        "a payload type must not be excused by the diagnostic exemption"
    );
    assert!(
        !hostile_payloads("pub struct ErrorBudget { pub label: &'static str }").is_empty(),
        "the exemption matches a trailing `Error`, not the substring anywhere"
    );
}
