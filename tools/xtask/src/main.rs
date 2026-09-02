mod architecture;

use std::env;

fn usage() -> ! {
    eprintln!("usage: cargo xtask architecture <check|list|graph|docs>");
    eprintln!("       cargo xtask architecture closure <check|docs|explain <profile>>");
    std::process::exit(2);
}

fn main() {
    let mut args = env::args().skip(1);
    if args.next().as_deref() != Some("architecture") {
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
    if let Err(error) = result {
        eprintln!("architecture: {error}");
        std::process::exit(1);
    }
}
