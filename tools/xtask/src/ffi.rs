use cargo_metadata::MetadataCommand;
use std::fs;
use std::path::PathBuf;

fn root() -> Result<PathBuf, String> {
    let metadata = MetadataCommand::new()
        .no_deps()
        .exec()
        .map_err(|error| format!("cargo metadata failed: {error}"))?;
    Ok(metadata.workspace_root.into_std_path_buf())
}

fn canonicalize_line_endings(bytes: Vec<u8>) -> Result<Vec<u8>, String> {
    let text = String::from_utf8(bytes)
        .map_err(|error| format!("cbindgen emitted non-UTF-8 output: {error}"))?;
    Ok(text.replace("\r\n", "\n").into_bytes())
}

fn rendered_header() -> Result<Vec<u8>, String> {
    let root = root()?;
    let crate_dir = root.join("crates/facade/axiolid-capi");
    let config = cbindgen::Config::from_file(crate_dir.join("cbindgen.toml"))
        .map_err(|error| error.to_string())?;
    let bindings = cbindgen::Builder::new()
        .with_crate(&crate_dir)
        .with_config(config)
        .generate()
        .map_err(|error| error.to_string())?;
    let mut bytes = Vec::new();
    bindings.write(&mut bytes);
    canonicalize_line_endings(bytes)
}

pub fn header() -> Result<(), String> {
    let path = root()?.join("crates/facade/axiolid-capi/include/axiolid.h");
    fs::write(&path, rendered_header()?).map_err(|error| error.to_string())?;
    println!("generated {}", path.display());
    Ok(())
}

pub fn check() -> Result<(), String> {
    let path = root()?.join("crates/facade/axiolid-capi/include/axiolid.h");
    let committed = canonicalize_line_endings(fs::read(&path).map_err(|error| error.to_string())?)?;
    let rendered = rendered_header()?;
    if committed != rendered {
        return Err(format!(
            "{} is stale; run `cargo xtask ffi header`",
            path.display()
        ));
    }
    println!("C header is current");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::canonicalize_line_endings;

    #[test]
    fn generated_header_has_platform_independent_line_endings() {
        assert_eq!(
            canonicalize_line_endings(b"first\r\nsecond\n".to_vec()).unwrap(),
            b"first\nsecond\n"
        );
    }
}
