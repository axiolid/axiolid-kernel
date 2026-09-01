use cargo_metadata::{Metadata, MetadataCommand, Package};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

pub type Result<T> = std::result::Result<T, String>;

#[derive(Debug, Clone)]
pub struct PackageArchitecture {
    pub name: String,
    pub path: String,
    pub layer: String,
    pub role: String,
    pub domain: String,
    pub public: bool,
    pub format_neutral: bool,
    pub allowed_internal_dependencies: BTreeSet<String>,
    pub actual_internal_dependencies: BTreeSet<String>,
}

#[derive(Debug)]
pub struct Architecture {
    pub root: PathBuf,
    pub packages: BTreeMap<String, PackageArchitecture>,
}

impl Architecture {
    pub fn load() -> Result<Self> {
        let metadata = MetadataCommand::new()
            .no_deps()
            .exec()
            .map_err(|error| format!("cargo metadata failed: {error}"))?;
        Self::from_metadata(metadata)
    }

    fn from_metadata(metadata: Metadata) -> Result<Self> {
        let root = PathBuf::from(metadata.workspace_root.as_str());
        let workspace: BTreeMap<String, &Package> = metadata
            .workspace_packages()
            .into_iter()
            .map(|package| (package.name.to_string(), package))
            .collect();
        let workspace_names: BTreeSet<String> = workspace.keys().cloned().collect();
        let mut packages = BTreeMap::new();

        for (name, package) in workspace {
            let table = package
                .metadata
                .get("axiolid")
                .and_then(Value::as_object)
                .ok_or_else(|| format!("{name}: missing [package.metadata.axiolid]"))?;
            let version = integer(table, "architecture-version", &name)?;
            if version != 1 {
                return Err(format!(
                    "{name}: unsupported architecture-version {version}"
                ));
            }
            let manifest_dir = Path::new(package.manifest_path.as_str())
                .parent()
                .ok_or_else(|| format!("{name}: manifest has no parent"))?;
            let path = manifest_dir
                .strip_prefix(&root)
                .map_err(|_| format!("{name}: package is outside workspace root"))?
                .to_string_lossy()
                .replace('\\', "/");
            let allowed_internal_dependencies =
                strings(table, "allowed-internal-dependencies", &name)?;
            let actual_internal_dependencies = package
                .dependencies
                .iter()
                .map(|dependency| dependency.name.to_string())
                .filter(|dependency| workspace_names.contains(dependency))
                .collect();
            packages.insert(
                name.clone(),
                PackageArchitecture {
                    name: name.clone(),
                    path,
                    layer: string(table, "layer", &name)?,
                    role: string(table, "role", &name)?,
                    domain: string(table, "domain", &name)?,
                    public: boolean(table, "public", &name)?,
                    format_neutral: boolean(table, "format-neutral", &name)?,
                    allowed_internal_dependencies,
                    actual_internal_dependencies,
                },
            );
        }
        Ok(Self { root, packages })
    }
}

fn value<'a>(
    table: &'a serde_json::Map<String, Value>,
    key: &str,
    package: &str,
) -> Result<&'a Value> {
    table
        .get(key)
        .ok_or_else(|| format!("{package}: missing architecture metadata key `{key}`"))
}

fn string(table: &serde_json::Map<String, Value>, key: &str, package: &str) -> Result<String> {
    value(table, key, package)?
        .as_str()
        .filter(|item| !item.trim().is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| format!("{package}: `{key}` must be a non-empty string"))
}

fn integer(table: &serde_json::Map<String, Value>, key: &str, package: &str) -> Result<u64> {
    value(table, key, package)?
        .as_u64()
        .ok_or_else(|| format!("{package}: `{key}` must be an unsigned integer"))
}

fn boolean(table: &serde_json::Map<String, Value>, key: &str, package: &str) -> Result<bool> {
    value(table, key, package)?
        .as_bool()
        .ok_or_else(|| format!("{package}: `{key}` must be a boolean"))
}

fn strings(
    table: &serde_json::Map<String, Value>,
    key: &str,
    package: &str,
) -> Result<BTreeSet<String>> {
    value(table, key, package)?
        .as_array()
        .ok_or_else(|| format!("{package}: `{key}` must be an array"))?
        .iter()
        .map(|item| {
            item.as_str()
                .map(ToOwned::to_owned)
                .ok_or_else(|| format!("{package}: `{key}` entries must be strings"))
        })
        .collect()
}
