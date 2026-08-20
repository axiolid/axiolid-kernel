# axiolid-heal instructions

Purpose: Explicit diagnosis and opt-in repair.

Allowed internal dependencies: axiolid-core. Follow parent `../AGENTS.md`. Do not read
`PLAN.md` unless assigned implementation or roadmap work.

## Module ownership

diagnosis.rs; repair.rs; traits.rs. Split a module before unrelated data, validation, and algorithms grow
together. Add no empty placeholder files.

## Invariants

Diagnosis never mutates. There is no repair-all switch. Every repair report records what changed and what remains.

Public values derive `Debug` and `Clone`; add other standard traits only when
semantically valid. Validate unsupported and unavailable paths in tests.
