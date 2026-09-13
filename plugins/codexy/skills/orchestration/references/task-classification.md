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
