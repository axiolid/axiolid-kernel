# C ABI closure probe

This standalone fixture freezes the internal Rust closure of the native C ABI. The executable only exercises the version symbol; the actual C compiler/link/runtime probe lives in `crates/facade/axiolid-capi/tests/c`.
