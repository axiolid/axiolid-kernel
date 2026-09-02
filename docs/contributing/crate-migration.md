# Crate migration guide

Pre-1.0 the kernel reorganised its flat `crates/axiolid-*` layout into
ownership-named directories. Package names changed with it.

A consumer pinned to a revision from before that change gets a hard resolve
failure, not a compile error: Cargo cannot find the package at all. This page
maps every removed name to where it went.

## Renamed packages

| Old package | Now |
| --- | --- |
| `axiolid-scalar` | `axiolid-reference`; exact predicates in `axiolid-predicates` |
| `axiolid-boolmesh` | `axiolid-mesh-boolean-boolmesh` |
| `axiolid-compile` | `axiolid-mesh-compile` |
| `axiolid-tessellate` | `axiolid-tessellation-contract` |
| `axiolid-kernel` | `axiolid-contracts`, `axiolid-guarantees`, `axiolid-dispatch` |

## Removed packages

| Old package | Disposition |
| --- | --- |
| `axiolid-sweep` | Removed. Held one `Sweeper` trait with no implementors. Sweep construction now lives in `axiolid-construct` (`fixed_reference_sweep`, `swept_disk`, `surface_curve_sweep`). |

## Directory layout

Package names no longer match directory names. Resolve a package by name, not
by path:

```
cargo metadata --no-deps --format-version 1 | jq -r ".packages[].name"
```

The directory holding a package may differ from the package name: the
`crates/algorithms/sampled/field/` directory publishes `axiolid-field-ops`.
