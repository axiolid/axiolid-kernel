# Axiolid C ABI

This crate is the only in-tree unsafe language boundary. It wraps the supported `axiolid::application` facade with versioned C symbols, scalar handles, `#[repr(C)]` data, bounded copies, and panic containment. Keep C/C++ and downstream-format types out of kernel crates. Generate `include/axiolid.h` from the Rust surface; never hand-maintain a second ABI schema.
