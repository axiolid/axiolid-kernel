# Where things go

Axiolid tracks its state on GitHub, not in prose. This page says which surface
owns what, so a contribution lands somewhere it can actually be acted on.

## The one distinction that matters

Two different things get confused constantly, so we split them deliberately:

| You want | Cost to you today | Where | Why there |
| --- | --- | --- | --- |
| Something the kernel **cannot do** | You are blocked | [Features discussion](https://github.com/axiolid/kernel/discussions/categories/features) | Scope and design must be agreed before code |
| Existing behavior to be **faster or lighter** | Milliseconds | [Optimization discussion](https://github.com/axiolid/kernel/discussions/categories/optimization) | Needs a measurement, and competes against capability work |
| Existing behavior is **awkward to use** | Keystrokes | [Papercut issue](https://github.com/axiolid/kernel/issues/new?template=papercut.yml) | Small, obvious, no design debate |
| Something is **wrong** | Correctness | [Bug issue](https://github.com/axiolid/kernel/issues/new?template=bug_report.yml) | Reproducible defect |
| To understand something | Confusion | [Q&A discussion](https://github.com/axiolid/kernel/discussions/categories/q-a) | Answers get marked and reused |

**Optimization vs papercut** is the pair people get wrong most:

- *Optimization* changes **how fast** something runs. It is a discussion because
  it needs a same-harness measurement, and because spending effort on speed is a
  trade-off against building capability
  ([ADR 0013](../adr/0013-deferred-performance-techniques.md) parks broad
  performance work deliberately).
- *Papercut* changes **how it feels** to call. It is an issue because the fix is
  small and uncontroversial.

A rename is a papercut. A cache is an optimization. Neither is a feature.

## Why requests start as discussions

An issue is a commitment to do work. A discussion is a place to decide whether
work should happen. Filing "please add X" as an issue skips that decision and
leaves a tracker full of things nobody agreed to.

So the flow is:

```
discussion ──accepted──▶ issue ──▶ milestone ──▶ PR ──▶ closed with evidence
     │
     └──declined──▶ closed, with the reasoning written down
```

Nothing is silently dropped. A declined request keeps its thread and gets
`decision:declined` with the reason, so the next person who wants it can read
why it did not happen and argue against the actual argument.

## Decision labels

| Label | Meaning |
| --- | --- |
| `decision:accepted` | In scope. Becomes an issue and is planned. |
| `decision:parked` | In scope but deliberately deferred. The unblock condition is recorded. |
| `decision:declined` | Not doing it. The reasoning is on the thread. |
| `needs-evidence` | The claim needs a repro or a same-harness measurement before it moves. |

## Milestones and the board

Milestones are **capability gates, not dates**. Each has exit criteria you can
check. Minor milestones can be inserted between them at any time.

The [project board](https://github.com/orgs/axiolid/projects/1) carries the
workflow: `Backlog → Ready → In Progress → In Review → Blocked → Done`. The
`Backlog — accepted, unscheduled` milestone holds work that is wanted but not
committed to a release; planning pulls from there.

## What still lives in docs

Docs explain **how the kernel works and why it is shaped that way** —
architecture, ADRs, capability status. GitHub tracks **what is being done and
what was decided**. When they disagree about state, GitHub wins.
