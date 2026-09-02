# architecture/

Machine-checked architecture declarations.

`closure-profiles.toml` declares minimal downstream dependency closures. Each
profile names an isolated consumer fixture under `tests/consumers/`, the exact
internal packages that must be present, and the packages that must be absent.

Verify with:

```bash
cargo xtask architecture closure check
cargo xtask architecture closure explain <profile>
```

A closure change is an API change. Update `expected_internal` deliberately and
record the reason in an ADR — never to silence a failing gate.
