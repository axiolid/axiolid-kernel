# Architecture documentation

This directory contains generated and hand-authored architecture maps for the physical workspace, package-role DAG, and capability/provider ownership.

- `current-target-crate-map.md` is the migration truth and conflict register.
- Generated maps must come from `cargo metadata` through `cargo xtask architecture docs`.
- Do not claim runtime capability from package metadata; implementation plus conformance is authoritative.
- Keep source-format, MCS/Pkl, Protobuf, and vendor DTOs outside Axiolid contracts.
