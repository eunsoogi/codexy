# Task classification

Before selecting an execution or recursion strategy, MUST consider difficulty,
verifiability, duration, reversibility, and context sufficiency using
[adaptive work](adaptive-work.md). These are decision inputs, not new runtime
classification fields or additions to the ownership record below. MUST NOT
require a report or file for every request. When evidence changes a property,
MUST revise the affected decisions without resetting ownership or budgets.

## Orchestration-request ownership

When the user asks to orchestrate work, the parent MUST keep only decomposition,
child selection and assignment, owner/dependency/resource decisions, minimum
coordination readbacks, result comparison and acceptance, next directions, and
user reporting. Every independent investigation, research, source exploration,
diagnosis, design artifact, implementation or repair, test execution,
reproduction, and execution verification MUST be assigned to a bounded child.
Read-only scope, small size, and urgency MUST NOT exempt that work.

The parent MAY read the minimum current issue/task/branch/goal state and
existing artifacts, diffs, or verification evidence needed for coordination and
acceptance. Acceptance verification means judging those results against the
criteria; execution verification means producing proof by investigation,
reproduction, or running checks. If a readback grows into that work, the parent
MUST separate and delegate it immediately. Insufficient or conflicting child
evidence MUST return to the existing owner or a suitable bounded investigation
child for supplementation; the parent MUST NOT silently investigate, repair, or
run the missing checks.

This boundary applies to an orchestration request, including read-only
orchestration. Ordinary single investigation or implementation requests MUST
retain their current-task route. Loading this skill MUST NOT itself select the
coordinator role, and an assigned implementation Worker MUST retain its lane. An
explicit plan-only request MUST retain its no-child/no-execution boundary.
Delegation intent MUST be honored within current user and host authority; it
MUST NOT manufacture task creation, messaging, goal, merge, or publication
rights. MUST use
[child routing](child-routing-policy.md#orchestration-request-dispatch) for
reuse, capacity, unavailable surfaces, and blocked children.

## Issue-owned implementation decisions

For a child-owned issue, the assigned outcome, scope and exclusions, acceptance
criteria, and applicable user, repository, and host constraints define the
Worker's boundaries. The assignment MUST state the outcome and limits without
prescribing a patch recipe.

Within those boundaries, the owning Worker MUST investigate and diagnose the
current state, design and choose the concrete solution and files, implement it,
run required checks, and choose repairs for actionable review findings. The
Worker MUST NOT need a parent-selected file or edit list, or new approval for
ordinary in-scope choices. The Orchestrator MAY impose path or resource limits
only to honor a user, repository, or host constraint or reserve another owner's
work; the assignment MUST state the reason and MUST NOT imply target files or
edits beyond that constraint.

The Orchestrator MUST keep one issue owner, coordinate dependencies and
shared-path ownership, compare returned evidence with acceptance criteria, and
retain merge and publication authority. Parent feedback MUST identify an unmet
criterion or verified ownership, safety, or dependency constraint, plus the
evidence gap. The Orchestrator MUST route actionable review findings to the
owning Worker, who MUST choose the repair. A parent diagnosis without
supporting evidence MUST be presented as a hypothesis for the Worker to assess,
not an edit directive.

On a shared-path conflict, cross-lane finding, or discovery outside the issue,
the Worker MUST pause only the affected change and report its path/finding,
evidence, and decision needed. The Orchestrator MUST resolve ownership or
scope/acceptance; the Worker MUST retain its lane and resume after clarification.
A discovery MUST NOT expand the Worker's issue or transfer its implementation to
the parent.

### Coordination and execution examples

These are document scenarios, not evidence that a host executed the routes.

| Request or signal                                 | Required division of work                                                                                                                              |
| ------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------ |
| “Orchestrate investigation and implementation.”   | Parent assigns child objectives and dependencies; children investigate, design, implement, and run verification.                                       |
| “Orchestrate read-only research.”                 | A bounded child researches and returns sources/results; parent compares and reports them.                                                              |
| “Investigate this failure.”                       | Ordinary single-task investigation stays with the assigned current task.                                                                               |
| Existing usable child owner                       | Parent reads owner/task state and reuses that owner; it does not create a replacement or patch its branch.                                             |
| Child says tests passed without adequate evidence | Parent compares the supplied proof with criteria and directs the owner to supply the missing run/result; it does not run the tests itself.             |
| Eight supervised slots occupied                   | Parent keeps unstarted work in backlog until authoritative capacity release; it does not execute the work itself.                                      |
| Required child surface or authority unavailable   | Parent records the exact limitation and needed owner/authority decision; execution remains pending without a parent or CLI/app-server fallback.        |
| Child reports blocked                             | Parent follows the Advisor-assisted supervision route, judges advice and directs the existing child; the child executes and returns recovery evidence. |

## Ownership record

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
