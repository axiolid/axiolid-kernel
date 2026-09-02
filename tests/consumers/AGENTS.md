# Closure fixtures

Each directory is an isolated downstream application used to verify one profile
in `architecture/closure-profiles.toml` (ADR 0036).

## Rules

- Every fixture has an EMPTY `[workspace]` table. It must be its own workspace
  root, otherwise workspace feature unification masks what a real consumer
  resolves and the measurement becomes meaningless.
- Depend on leaf packages by relative path with `default-features = false`.
- `Cargo.lock` is gitignored per fixture; the checker resolves with `--offline`.
- Exercise real behaviour, not just symbol names. A fixture that only mentions a
  type can pass while the package is unusable.
- Adding a dependency here changes a declared compatibility promise. Update the
  profile deliberately and regenerate the closure docs.

## Verify

```bash
cargo xtask architecture closure check
cargo xtask architecture closure explain <profile>
bash scripts/probe_closure_gate.sh
```
