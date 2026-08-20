# Axiolid

Axiolid is a pure-Rust, format-agnostic geometry kernel. It preserves exact geometric intent in a neutral DAG, keeps topology and geometry distinct, and exposes operation contracts separately from CPU and GPU execution providers.

## Packages

- `axiolid-core`: f64 values, transforms, bounds, and tolerance policy.
- `axiolid-model`: immutable geometry DAG.
- representation, algorithm, contract, and backend crates: opt-in capabilities behind the `axiolid` facade.

See `AGENTS.md`, `docs/adr/`, and `scripts/gate.sh` for architecture and verification.
