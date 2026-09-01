use super::model::{Architecture, Result};
use std::fs;
use std::path::{Path, PathBuf};

// Production source is intentionally format-neutral. Tests may name external
// formats when proving that adapters and architecture gates reject leakage.
const FORBIDDEN_FORMAT_TERMS: &[&str] = &[
    "ifc", "revit", "solibri", "cset", "pkl", "protobuf", "prost",
];
const FORBIDDEN_NATIVE_DEPENDENCIES: &[&str] = &["bindgen", "cxx", "autocxx", "cmake"];

pub fn validate(architecture: &Architecture, errors: &mut Vec<String>) -> Result<()> {
    for package in architecture.packages.values() {
        if package.layer == "tools" {
            continue;
        }
        let src = architecture.root.join(&package.path).join("src");
        if !src.exists() {
            errors.push(format!("{}: missing src directory", package.name));
            continue;
        }
        let files = rust_files(&src)?;
        for file in &files {
            let source = fs::read_to_string(file)
                .map_err(|error| format!("read {}: {error}", file.display()))?;
            let lower = source.to_ascii_lowercase();
            if package.format_neutral {
                for term in FORBIDDEN_FORMAT_TERMS {
                    if lower.contains(term) {
                        errors.push(format!(
                            "{}: {} contains forbidden source-format term `{term}`",
                            package.name,
                            relative(&architecture.root, file)
                        ));
                    }
                }
            }
            for placeholder in ["todo!(", "unimplemented!("] {
                if source.contains(placeholder) {
                    errors.push(format!(
                        "{}: {} contains panicking placeholder `{placeholder}`",
                        package.name,
                        relative(&architecture.root, file)
                    ));
                }
            }
            validate_declared_module(&src, file, &source, errors)?;
        }
        let manifest = fs::read_to_string(architecture.root.join(&package.path).join("Cargo.toml"))
            .map_err(|error| format!("read {} manifest: {error}", package.name))?;
        for dependency in FORBIDDEN_NATIVE_DEPENDENCIES {
            if manifest.lines().any(|line| {
                let line = line.trim_start();
                !line.starts_with('#') && line.starts_with(dependency)
            }) {
                errors.push(format!(
                    "{}: forbidden native bridge dependency `{dependency}`",
                    package.name
                ));
            }
        }
        let forbids_unsafe = matches!(
            package.layer.as_str(),
            "foundation" | "representations" | "contracts" | "facade"
        ) || package.name == "axiolid-backend-gpu";
        if forbids_unsafe {
            let lib = src.join("lib.rs");
            let source = fs::read_to_string(&lib)
                .map_err(|error| format!("read {}: {error}", lib.display()))?;
            if !source.contains("#![forbid(unsafe_code)]") {
                errors.push(format!(
                    "{}: {} must forbid unsafe code",
                    package.name,
                    relative(&architecture.root, &lib)
                ));
            }
        }
    }
    Ok(())
}

fn rust_files(root: &Path) -> Result<Vec<PathBuf>> {
    fn visit(dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
        for entry in
            fs::read_dir(dir).map_err(|error| format!("read {}: {error}", dir.display()))?
        {
            let path = entry
                .map_err(|error| format!("read directory entry: {error}"))?
                .path();
            if path.is_dir() {
                visit(&path, files)?;
            } else if path.extension().and_then(|value| value.to_str()) == Some("rs") {
                files.push(path);
            }
        }
        Ok(())
    }
    let mut files = Vec::new();
    visit(root, &mut files)?;
    files.sort();
    Ok(files)
}

fn validate_declared_module(
    src: &Path,
    file: &Path,
    _source: &str,
    errors: &mut Vec<String>,
) -> Result<()> {
    if file.file_name().and_then(|value| value.to_str()) == Some("lib.rs")
        || file
            .parent()
            .and_then(Path::file_name)
            .and_then(|value| value.to_str())
            == Some("bin")
        || file.components().any(|part| part.as_os_str() == "tests")
    {
        return Ok(());
    }
    let stem = file
        .file_stem()
        .and_then(|value| value.to_str())
        .ok_or_else(|| format!("{} has no UTF-8 module stem", file.display()))?;
    let parent = file
        .parent()
        .ok_or_else(|| format!("{} has no parent", file.display()))?;
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
    let declaring_source = fs::read_to_string(&declaration)
        .map_err(|error| format!("read {}: {error}", declaration.display()))?;
    if !declaring_source.contains(&format!("mod {stem};"))
        && !declaring_source.contains(&format!("pub mod {stem};"))
    {
        errors.push(format!(
            "{} is not declared by {}",
            file.display(),
            declaration.display()
        ));
    }
    Ok(())
}

fn relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}
