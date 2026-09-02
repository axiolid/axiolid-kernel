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
| **Field** `Area` | Which part of the kernel? | `exact-brep`, `intersection`, `nurbs`, `mesh`, `architecture`, `release`, `docs` |
| **Field** `Effort` | How much work is it? | `S`, `M`, `L`, `XL` |
| **Field** `Priority` | How urgent is it? | `P0` … `P3`, derived from the milestone |
| **Milestone** | Which release gate? | `v0.2` … `v1.0`, or `Backlog` |
| **Board status** | Where is it right now? | `Backlog` → `Ready` → `In Progress` → `In Review` → `Blocked` → `Done` |
| **Close reason** | Why did it end? | `Completed`, `Not planned`, `Duplicate` |

`Area` and `Effort` are **project fields**, not labels. Fields are sortable,
groupable, and chartable on the board; labels are none of those. They were
labels first, and that was the wrong call — a label is a flat tag, whereas
"how much work" is a value you want to group and sum by.

There is deliberately **no `bug` label** — that is the `Bug` *type*. Likewise no
`enhancement` label: new capability is the `Feature` type, and it starts as a
discussion. Duplicating a type as a label means filters silently miss things.

### Accepting and rejecting

There is no `decision:accepted` label either, because GitHub already says this
natively and more precisely:

| Decision | How it is recorded | Where the reasoning lives |
|---|---|---|
| **Accepted** | The issue exists, is on the board, and has a milestone | the thread |
| **Declined** | Closed as **Not planned** | the closing comment |
| **Duplicate** | Closed as **Duplicate**, linked to the original | the link |
| **Answered** (discussions) | Comment **marked as answer** | the answer |
| **Deferred** | `parked` label + `Blocked` on the board | the unblock condition |

An accepted issue *is* an accepted issue — labelling it as such adds nothing a
filter cannot already see. Closing as **Not planned** is stronger than a
`decision:declined` label: it renders differently, it is filterable
(`is:closed reason:not-planned`), and it survives label churn.

`parked` is the one that stays, because it is the only state GitHub cannot
express: in scope, agreed, deliberately not now. That is not "closed" and not
"ready" — it needs a name.

## From discussion to issue

An accepted discussion becomes an issue via **Create issue from discussion** in
the discussion sidebar. This copies the body, links both directions, and leaves
the discussion in place as the rationale.

That path does **not** go through an issue template, so nothing is pre-filled.
Automation classifies it from the discussion's category instead:

| Category | Becomes | Why |
|---|---|---|
| 🚀 Features | type `Feature` | behavior that does not exist yet |
| ⚡ Optimization | type `Optimization` | same behavior, cheaper |
| ❓ Questions | type `Task` | usually a docs or ergonomics follow-up |

Features and Optimization are deliberately different types, not one bucket:
"make it do X" and "make X cost less" are scheduled, measured, and accepted on
different evidence. An optimization needs a measurement; a feature needs a use
case.

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

Milestones are a third case: they carry **no list at all**. A milestone
description is one or two lines stating the broad goal. The issues assigned to
it *are* the exit criteria, and GitHub's progress bar tracks them as they
close. Restating criteria in the description would duplicate that and go stale
— and checkboxes there render `disabled`, so they can never be ticked anyway.

> GitHub `[tasklist]` blocks were retired on 2025-04-30. Sub-issues replace
> them; a plain `- [ ]` list in an *issue* body is still interactive and can
> be converted to sub-issues in place.

## What happens after you file

1. It gets a type, an owner, and `needs-triage` — automatically, even if you
   filed through the API.
2. A maintainer decides. **Nothing is silently dropped:** a rejected request is
   closed as **Not planned** with the reason in the closing comment, so the next
   person argues with the reasoning instead of repeating the request.
3. Accepted work gets an `Area`, an `Effort`, and a milestone, then appears on
   the [project board](https://github.com/orgs/axiolid/projects/1).
4. Accepted discussions become issues. The discussion stays as the rationale.

## Evidence

Two claims need evidence before they can be accepted:

- **A bug** needs a reproduction and the commit you tested against.
- **An optimization** needs a measurement from the same harness, before and
  after. "Should be faster" is not a measurement.

Requests missing these get `needs-evidence` rather than being closed.
