# tools/fixtures

The shared adversarial and degenerate geometry corpus (#18). Consumed by the
differential tests in `axiolid-mesh-boolean-boolmesh` and by any crate that
needs a case known to break naive implementations.

## Why constructed, not stored

A degenerate case is usually a *number*, not a file: a 2e-9 plane tilt, a
sliver a fraction of a millimetre wide, two vertices that coincide exactly.
Round-tripping those through a mesh file format invites an exporter to round a
coordinate, at which point the fixture silently stops being degenerate and the
test keeps passing while covering nothing.

Constructing in code keeps the exact bit pattern under version control.

## Adding a fixture

1. Write a constructor returning `Fixture`.
2. Fill in every `Provenance` field. `source` must be specific enough that a
   reader can go and verify the claim -- an issue number, an ADR, or the
   geometric principle involved.
3. State `expectation` as what an implementation *must* do. Never record what
   it currently does: a fixture that encodes present behaviour cannot detect a
   regression, because the regression just becomes the new expectation.
4. Add it to `corpus()` so existing differential tests pick it up without
   being edited.

## Licence

Every fixture here is original work, reconstructed from published bug
descriptions or first principles, and carries the repository licence. Nothing
is copied from an external corpus, which keeps redistribution unencumbered.
If that ever changes, the `licence` field must say so.

## Verify

```bash
cargo test -p axiolid-fixtures
```
