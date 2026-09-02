# Where things go

Axiolid tracks its state on GitHub, not in prose. This page says which surface
to use, so a request lands where it can actually be acted on.

## The one rule

**Discussions are for deciding. Issues are for doing.**

If the answer to "should we do this at all?" is not yet obvious, it is a
discussion. Once it is agreed, it becomes an issue with an owner.

## Choosing a surface

| You want to… | Surface | Why |
|---|---|---|
| 🚀 Ask for a capability the kernel lacks | [Features discussion](https://github.com/axiolid/kernel/discussions/new?category=features) | Scope needs agreeing before code |
| ⚡ Make existing behavior faster or lighter | [Optimization discussion](https://github.com/axiolid/kernel/discussions/new?category=optimization) | Needs a measurement, not an intuition |
| 💬 Ask how something works | [Question discussion](https://github.com/axiolid/kernel/discussions/new?category=q-a) | Answers get marked and found again |
| 🐛 Report wrong behavior | [Bug issue](https://github.com/axiolid/kernel/issues/new?template=1-bug.yml) | Concrete, reproducible, actionable |
| 🩹 Report awkward-but-correct API | [Papercut issue](https://github.com/axiolid/kernel/issues/new?template=2-papercut.yml) | Small fix, no design argument needed |
| 📖 Report wrong or missing docs | [Docs issue](https://github.com/axiolid/kernel/issues/new?template=3-docs.yml) | Especially overclaims |

### ⚡ Optimization vs 🩹 papercut

The distinction people get wrong most often:

- An **optimization** costs you **milliseconds**. It changes how fast existing,
  correct behavior runs. It is a *discussion*, because it needs a same-harness
  measurement before it is worth doing.
- A **papercut** costs you **keystrokes**. It changes how pleasant correct
  behavior is to call. It is an *issue*, because the fix is usually obvious.

A rename is a papercut. A cache is an optimization. Neither adds capability —
that is a feature request.

## Types, labels, and milestones

These answer different questions. Using them for the same thing makes both
useless.

| Mechanism | Answers | Values |
|---|---|---|
| **Type** | What kind of work is this? | `Bug`, `Task`, `Feature` |
| **Label** `area:*` | Which part of the kernel? | `exact-brep`, `intersection`, `nurbs`, `mesh`, `architecture`, `release` |
| **Label** `size:*` | How big is the change? | `small`, `big` |
| **Label** `decision:*` | What did we decide, and why? | `accepted`, `parked`, `declined` |
| **Milestone** | Which release gate? | `v0.2` … `v1.0`, or `Backlog` |
| **Board status** | Where is it right now? | `Backlog` → `Ready` → `In Progress` → `In Review` → `Blocked` → `Done` |

There is deliberately **no `bug` label** — that is the `Bug` *type*. Likewise no
`enhancement` label: new capability is the `Feature` type, and it starts as a
discussion. Duplicating a type as a label means filters silently miss things.

## ☑️ Checklists vs 🧩 sub-issues

Both express "this has parts". They are not interchangeable.

Use a **checklist** (`- [ ]`) when the parts are:

- one person, one sitting, one pull request;
- acceptance criteria rather than separately schedulable work;
- meaningless outside this issue ("update the changelog", "add a test").

Open **sub-issues** when a part is:

- independently assignable — someone else could take it in parallel;
- landing in its own pull request;
- worth its own status on the board;
- worth its own type (a `Feature` whose child is a `Task`).

> **Rule of thumb:** if someone else could pick it up *right now, in parallel*,
> it is a sub-issue. If it only makes sense as a step inside this change, it is
> a checkbox.

Sub-issues give a progress bar on the parent and keep the board honest, because
each child carries real status. A checklist inside a six-month umbrella issue
tells you nothing — that is what a
[🧭 tracking issue](https://github.com/axiolid/kernel/issues/new?template=5-tracking.yml)
with sub-issues is for.

## What happens after you file

1. It gets a type, an owner, and `needs-triage` — automatically, even if you
   filed through the API.
2. A maintainer applies a `decision:` label. **Nothing is silently dropped:**
   `decision:declined` keeps the thread and records why, so the next person
   argues with the reasoning instead of repeating the request.
3. Accepted work gets an `area:`, a `size:`, and a milestone, then appears on
   the [project board](https://github.com/orgs/axiolid/projects/1).
4. Accepted discussions become issues. The discussion stays as the rationale.

## Evidence

Two claims need evidence before they can be accepted:

- **A bug** needs a reproduction and the commit you tested against.
- **An optimization** needs a measurement from the same harness, before and
  after. "Should be faster" is not a measurement.

Requests missing these get `needs-evidence` rather than being closed.
