# Task classification

Before selecting an execution or recursion strategy, MUST consider difficulty,
verifiability, duration, reversibility, and context sufficiency using
[adaptive work](adaptive-work.md). These are decision inputs, not new runtime
classification fields or additions to the ownership record below. MUST NOT
require a report or file for every request. When evidence changes a property,
MUST revise the affected decisions without resetting ownership or budgets.

Durable delegation and multi-lane ownership MUST emit the following record so
ownership, scope, worktree, and handoff remain explicit. Those collaboration
signals alone do not select the strict workflow profile. Explicit audit evidence
and concrete strict-risk work MUST also emit:

```text
Ownership metadata source: parent-supplied
Lane ownership: child-owned
Task classification:
```

GFM `| Field | Value |`, `| --- | --- |`; non-empty rows: Lane type; Secondary
surfaces; Owner decision; Atomic scope; Required skills; Required
tools/evidence; First allowed action; Stop/blocker. Owner decision MUST affirm
metadata. Current-thread-classified may own
parent-owned/child-owned/current-thread-owned/external/human-owned;
parent-supplied permits only child-owned.
