# 0038 — Intersection curves are constructed only where they are exact

- **Status:** Accepted
- **Date:** 2026-09-03
- **Issue:** [#6](https://github.com/axiolid/kernel/issues/6)

## Context

Certified intersection proved WHERE surfaces meet — parameter boxes, endpoint
enclosures, transversality bounds — but never returned a curve. Exact booleans,
analytic section curves, offsets, and fillets all need the curve itself, so
this was the gate in front of all of them.

The tempting shortcut is to march along the certified boxes and emit a
polyline. That produces a curve for every input, which is precisely why it is
wrong: the output would carry the word "certified" while being a tessellation
whose error nothing bounds.

## Decision

Construct a curve only where the construction is EXACT, and refuse by name
everywhere else.

For two exact single-span affine patches — the family the trace certificate
already establishes — the intersection is a straight line. The degree-1
segment between the two certified endpoints IS that line, not a sampling of
it.

The deviation bound is valid over the whole curve, and this is the load-bearing
argument:

- each affine patch lies in a plane, and signed distance to a plane is an
  affine function of position;
- an affine function on a segment attains its extremes at the endpoints;
- therefore a residual bound proven at both endpoints bounds the entire
  segment.

Without that argument the bound would only describe the two points that
happened to be checked.

Curve/surface intersection returns POINTS, not curves. A transverse crossing is
isolated; threading a curve through isolated hits would assert an extent
nothing proved. A curve arises only when the curve lies IN the surface, a
coincident case this certification does not cover and therefore refuses.

Refusals are `Ok(Err(..))`, not `Err`. "These surfaces do not meet" and "this
shape is not provable here" are answers about geometry, not failures. `Err`
stays reserved for invalid input and exhausted budgets.

`Disjoint` is deliberately distinct from `Unresolved`: proven absence is not
the same fact as undecided, and collapsing them would let a caller read "we
could not tell" as "there is nothing there".

If any trace in a query is not constructible the whole query refuses. Returning
the provable subset would silently under-report the intersection, which for a
boolean is worse than refusing outright.

## Consequences

The supported family is narrow: plane/plane surface intersections and
transverse curve/surface crossings. Curved patch pairs, coincident and
tangential contacts, and multispan traces all refuse.

That is the honest state of the kernel rather than a limitation introduced
here, and it is now visible in the type system instead of implied by prose.
Widening the family means proving a new construction, not loosening a check.

Verification follows ADR 0037: constructed curves are sampled and checked
against both surfaces by `axiolid-oracle` in mapped 3D, and every refusal
variant has its own fixture, so the refusal paths are tested rather than
assumed.
