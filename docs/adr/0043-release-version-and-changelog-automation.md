# 0043 — Release version/changelog rollover is a checked, reviewed script

- **Status:** Accepted
- **Date:** 2026-09-03
- **Deciders:** Friedrich Schrödter
- **Supersedes:** —

## Context

`scripts/publish-workspace.py` and `scripts/verify-packages.py` (ADR 0035) already
prove the crates.io publish *order* and *byte-identity*, gated in `scripts/gate.sh`
and `.github/workflows/publish.yml`. Neither script touches the two inputs every
release still required by hand: bumping `[workspace.package].version` (and every
internal `axiolid-*` path-dependency version requirement that pins to it in
`[workspace.dependencies]`), and rolling `docs/CHANGELOG.md`'s `## [Unreleased]`
section into a dated version heading. A hand-rolled bump is exactly the kind of
step that silently drifts: forgetting one of the ~38 internal dependency version
requirements leaves a workspace-internal crate unable to depend on the version
that will actually publish, because Cargo's default caret semantics on a 0.x
version (`version = "0.1.0"` means `^0.1.0`) reject `0.2.0`.

While wiring this, `scripts/verify-packages.py` was found already broken:
`axiolid-nurbs`'s `axiolid-oracle` dev-dependency (test-only, `publish = false`)
carried a pinned `version = "0.1.0"` in `[workspace.dependencies]`. Cargo can
strip a *versionless* path dev-dependency during `cargo package`, but not a
versioned one pointing at a crate that will never exist on crates.io — every
`cargo package --workspace` failed with `no matching package named
axiolid-oracle`. This was a latent regression in the existing gate, not new
scope; issue #10's verification requirement (`bash scripts/gate.sh` proves a
dry-run release) could not otherwise be satisfied.

## Decision

Add `scripts/prepare-release.py` as the version/changelog half of release
automation, and fix the `axiolid-oracle` dependency declaration so the existing
publish-order and package-preflight scripts actually run in `scripts/gate.sh`.

- `prepare-release.py --release X.Y.Z` (default `--check`, only `--write`
  mutates) validates a strictly-forward semver bump, refuses to release an
  empty `## [Unreleased]` section, dates and rolls that section into
  `## [X.Y.Z] - <date>`, and rewrites both `[workspace.package].version` and
  every internal `axiolid-*` dependency's `version = "<old>"` requirement in
  `[workspace.dependencies]` to the new version in one pass.
- `axiolid-oracle`'s workspace dependency entry drops its `version` field
  (dev-dependency only, never published); `axiolid-nurbs` keeps depending on
  it via `axiolid-oracle.workspace = true`, which now resolves the versionless
  path form.
- `scripts/gate.sh` runs `scripts/test_release_scripts.py` (which now also
  covers `prepare-release.py`), `scripts/publish-workspace.py` (plan-only, no
  token), and `scripts/verify-packages.py` (bootstrap preflight for all 38
  publishable archives) as ordinary gate steps.

## Alternatives considered

| Option | Why not |
| --- | --- |
| Adopt `release-plz` or `cargo-release` | Neither natively rewrites a workspace's own internal path-dependency version pins across every member in one atomic pass the way this workspace's `[workspace.dependencies]` layout requires; wrapping either would still need a bespoke bump step, at the cost of a new external dependency and its own trust/version surface. |
| Bump versions by hand per release | The exact failure mode this ADR exists to remove: a missed internal pin is invisible until a *dependent* crate publishes (Cargo does not check it at `cargo package --dry-run` time on a leaf). |
| Leave `verify-packages.py`/`publish-workspace.py` unwired from `gate.sh` | Leaves the release pipeline exercised only at `workflow_dispatch` time on `main`, where a regression is discovered during an actual release attempt instead of any ordinary commit. |

## Consequences

**Positive**

- A release bump is one reviewed, tested command instead of a hand-edited
  diff across `Cargo.toml` and `docs/CHANGELOG.md`.
- `scripts/gate.sh` — the same gate every other change goes through — now
  proves the publish order and package-preflight for the entire workspace,
  satisfying issue #10's "dry-run release proving the full workspace
  publishes in a valid order" verification requirement without waiting for a
  real release.
- The `axiolid-oracle` fix removes a genuine, previously undetected gate gap.

**Negative / costs**

- `prepare-release.py`'s dependency-version rewrite is a targeted regex over
  `[workspace.dependencies]`, not a full TOML edit; a workspace dependency
  line that does not match the `axiolid-<name> = { path = "...", version =
  "<current>" ... }` shape (for example, a non-`axiolid`-prefixed internal
  crate) would need the regex extended. This is documented in the script's
  own docstring; the fixed set of `axiolid-*` internal crates is checked by a
  count-of-replacements-made guard that fails closed at zero matches.

**Follow-ups / risks to watch**

- The actual `--execute` upload path is unchanged and still requires a human
  to invoke `workflow_dispatch` with `CARGO_REGISTRY_TOKEN`; this ADR covers
  only the version/changelog and dry-run-verification steps.

## Relation to existing code

- `scripts/prepare-release.py` (new)
- `scripts/test_release_scripts.py` (extended: `prepare-release.py` coverage,
  corrected a stale hardcoded publish-plan package count)
- `scripts/gate.sh` (extended: release script tests, publish plan, package
  preflight)
- `Cargo.toml`, `crates/algorithms/parametric/nurbs/Cargo.toml`
  (`axiolid-oracle` version-pin fix)
- `docs/CHANGELOG.md`
