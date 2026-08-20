# axiolid-scalar instructions

Purpose: portable scalar reference implementation and correctness oracle (ADR 0012).

Allowed internal dependencies: axiolid-core, axiolid-kernel. Follow parent `../AGENTS.md`. Do not read
`PLAN.md` unless assigned implementation or roadmap work.

## Module ownership

expansion.rs; orientation.rs. Split a module before unrelated data, validation, and algorithms grow
together. Add no empty placeholder files.

## Invariants

No intrinsics, no threading, no feature gates, no `unsafe`. This crate is the
differential oracle every optimized backend is validated against, so readability
outranks speed. Per ADR 0012 the scalar implementation of an operation lands
before any optimized implementation of it.

Predicates return `Certified`, never a bare sign. A predicate that can escalate
must escalate: returning an uncertified sign for a topology decision is the one
failure this crate exists to prevent. Error bounds scale with operand magnitude;
a constant epsilon is a bug.

Tests must include a differential gate against an oracle that shares no code
with the implementation, and must assert that the exact path was actually
reached -- a test suite that never escalates proves nothing about exactness.
