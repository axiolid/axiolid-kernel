mod architecture;
mod ffi;

use std::env;

fn usage() -> ! {
    eprintln!("usage: cargo xtask architecture <check|list|graph|docs>");
    eprintln!("       cargo xtask architecture closure <check|docs|explain <profile>>");
    eprintln!("       cargo xtask ffi <header|check>");
    std::process::exit(2);
}

fn main() {
    let mut args = env::args().skip(1);
    let command = args.next();
    if command.as_deref() == Some("ffi") {
        let result = match args.next().as_deref() {
            Some("header") => ffi::header(),
            Some("check") => ffi::check(),
            _ => usage(),
        };
        finish("ffi", result);
        return;
    }
    if command.as_deref() != Some("architecture") {
        usage();
    }
    let result = match args.next().as_deref() {
        Some("check") => architecture::check(),
        Some("list") => architecture::list(),
        Some("graph") => architecture::graph(),
        Some("docs") => architecture::docs(),
        Some("closure") => match args.next().as_deref() {
            Some("check") => architecture::closure_check(),
            Some("docs") => architecture::closure_docs(),
            Some("explain") => match args.next() {
                Some(profile) => architecture::closure_explain(&profile),
                None => usage(),
            },
            _ => usage(),
        },
        _ => usage(),
    };
    finish("architecture", result);
}

fn finish(command: &str, result: Result<(), String>) {
    if let Err(error) = result {
        eprintln!("{command}: {error}");
        std::process::exit(1);
    }
}
