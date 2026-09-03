# Downstream gate tests

Unit tests here validate the black-box consumer policy and manifest renderer used by `scripts/test-downstream-consumers.py`.

The executable Rust fixtures remain under `tests/consumers/`; the gate copies their sources into independent temporary workspaces, replaces development-only path dependencies with exact-version dependencies pinned to an immutable Git artifact, and inspects Cargo's resolved source identities.

## Verify

```bash
python3 -m unittest tests/downstream/test_downstream_consumers.py
python3 scripts/test-downstream-consumers.py
```
