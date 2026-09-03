use std::fs;
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("xtask lives under tools/xtask")
        .to_path_buf()
}

fn rendered_header() -> Result<Vec<u8>, String> {
    let root = root();
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
    Ok(bytes)
}

pub fn header() -> Result<(), String> {
    let path = root().join("crates/facade/axiolid-capi/include/axiolid.h");
    fs::write(&path, rendered_header()?).map_err(|error| error.to_string())?;
    println!("generated {}", path.display());
    Ok(())
}

pub fn check() -> Result<(), String> {
    let path = root().join("crates/facade/axiolid-capi/include/axiolid.h");
    let committed = fs::read(&path).map_err(|error| error.to_string())?;
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
