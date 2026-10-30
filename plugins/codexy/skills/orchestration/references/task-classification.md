# Task classification

Before selecting an execution or recursion strategy, MUST consider difficulty,
verifiability, duration, reversibility, and context sufficiency using
[adaptive work](adaptive-work.md). These are decision inputs, not new runtime
classification fields or additions to the ownership record below. MUST NOT
require a report or file for every request. When evidence changes a property,
MUST revise the affected decisions without resetting ownership or budgets.

## Orchestration-request ownership

This section is the canonical decision source for task kind, scoped authority,
implementation owner, and the distinction between planning dependencies and
execution conflicts. MUST apply it before choosing a route or ordering work.

### Task kind, authority, and owner

1. An explicit plan-only request MUST stay with `$planning`; it MUST NOT create
   an issue, child task, branch, worktree, or implementation. A plan MUST NOT
   grant execution authority or transfer an existing owner.
2. An ordinary single-task request MUST stay with its current assigned owner.
   When the current task is that owner, a need for a branch, worktree, or PR
   does not by itself authorize a separate app task. An assigned implementation
   Worker MUST retain its issue lane. An authorized atomic implementation MUST
   NOT require a plan file or new approval for ordinary in-scope choices.
3. An explicit request to orchestrate independent work means the parent MUST own
   decomposition, owner and dependency decisions, minimum coordination
   readbacks, result comparison and acceptance, next directions, and user
   reporting. The parent MUST assign each independent investigation, research,
   source exploration, diagnosis, design, implementation or repair, test
   execution, reproduction, and execution-verification outcome to a bounded
   child owner; small, urgent, and read-only work MUST be assigned as well.
   Direct readbacks MUST stay limited to current issue/task/goal/branch state
   and existing artifacts or verification evidence needed for coordination and
   acceptance. Acceptance means judging returned evidence against the criteria;
   execution verification means producing proof. If a readback becomes
   substantive research, diagnosis, design, implementation, test execution,
   reproduction, or execution verification, that work MUST stay with its
   assigned child or a bounded child assigned to supplement it. The parent MUST
   NOT silently perform it when child evidence is missing or conflicting.
   Independent outcomes MUST be split into atomic lanes before separate tasks,
   worktrees, branches, or PRs are assigned, unless a maintainer scopes them as
   one outcome.
4. Before dispatch, the parent MUST read back the active owner for the same
   issue-sized outcome and reuse a suitable existing owner. A helper MUST NOT
   replace, duplicate, or patch an implementation owner. MUST keep exactly one
   implementation owner for each issue-sized outcome. The parent MUST
   independently confirm the callable app-task tool and its invocation contract
   before using it. A source registration, GitHub assignee, branch, or issue
   alone MUST NOT prove tool availability or active ownership. A suitable active
   child MUST retain ownership; the parent MUST direct it through the supported
   task route and MUST NOT patch its lane or create a duplicate owner.
5. Scoped orchestration authority covers only the necessary bounded child tasks
   and task messages for the assigned outcome. An explicit user request to
   orchestrate a defined outcome authorizes those necessary bounded tasks and
   messages within its scope; the parent MUST NOT ask again per child or
   message. This authority MUST preserve explicit no-new-task and recipient
   limits, host and organization permissions, and existing ownership. It MUST
   NOT authorize merge, release, publication, or destructive actions. Vague
   coordination, issue complexity, branch/worktree need, or a parent handoff
   MUST NOT create that authority. A fresh-stage prerequisite MUST NOT grant an
   assigned candidate authority to create a nested task. A host denial MUST be
   preserved; Codex MUST NOT bypass it or invent permission parameters.
6. If the intended owner or topology is unresolved, a selected child-owned lane
   MUST remain pending when its owner, required app-task surface, or authority
   is unavailable. The parent MUST report the exact gap; it MUST NOT relabel the
   lane as parent-owned or current-task-owned to begin implementation, or
   substitute parent execution or an unsupported CLI/app-server route. For an
   ordinary single-task assignment, the current task MUST retain ownership when
   it is the assigned owner.

### Dependencies, reservations, and integration conflicts

- A planning dependency exists when a consumer needs an unavailable artifact,
  outcome, or contract decision. MUST put that prerequisite before its consumer.
- An execution reservation applies to a concrete resource that cannot be used
  concurrently, such as the same checkout, migration target, or Simulator. MUST
  gate work only while that resource is reserved; a list of overlapping paths is
  not a resource reservation.
- An integration conflict is an actual incompatible change that needs owner
  coordination. MUST resolve it in the owning branch before merge, preserve both
  intended changes, and rerun affected checks. Path overlap alone MUST NOT
  create a planning dependency or dispatch barrier. Dependency-ready lanes MUST
  remain concurrent within existing capacity.

Using a plugin or loading a skill MUST NOT promote an assigned Worker,
evaluator, or helper to orchestrator or expand its task-creation authority.
Durable delegation and multiple lanes alone MUST NOT select the strict profile;
follow [workflow profiles](workflow-profiles.md) for profile selection.

Planning owns plan content and ordering; MUST use its skill for plan-file rules.
Dispatch and setup mechanics remain in
[child routing](child-routing-policy.md#orchestration-request-dispatch) and
[thread/worktree routing](thread-and-worktree-routing.md). Parent-owned accepted
child cleanup MUST follow the
[canonical cleanup procedure](../../../../codexy-github/skills/git-workflow/references/local-git-and-branches.md#accepted-child-cleanup).

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

The Orchestrator MUST compare returned evidence with acceptance criteria and
retain merge and publication authority. Parent feedback MUST identify an unmet
criterion or verified ownership, safety, or dependency constraint, plus the
evidence gap. The Orchestrator MUST route actionable review findings to the
owning Worker, who MUST choose the repair. The Orchestrator MUST NOT patch a
child-owned lane unless an authorized owner change returns it to the current
task. A parent diagnosis without supporting evidence MUST be presented as a
hypothesis for the Worker to assess, not an edit directive.

On an actual integration conflict, cross-lane finding, or discovery outside the
issue, the Worker MUST pause only the affected change and report its
path/finding, evidence, and decision needed. The Orchestrator MUST resolve
ownership or scope/acceptance; the Worker MUST retain its lane and resume after
clarification. A discovery MUST NOT expand the Worker's issue or transfer its
implementation to the parent.

### Classification examples

These are document scenarios, not evidence that a host executed the routes.

| Request or signal                               | Classification and owner decision                                                                                                                                      |
| ----------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| “Plan this change; do not implement it.”        | MUST keep this plan-only; `$planning` owns the plan, with no issue execution or new task.                                                                              |
| “Implement this issue.”                         | The assigned current task MUST own the work unless an existing owner or explicit assignment says otherwise; a branch/worktree need alone MUST NOT create a child task. |
| “Orchestrate investigation and implementation.” | The parent MUST coordinate; each independent substantive outcome MUST go to one bounded owner under scoped authority and current host limits.                          |
| Vague request to “coordinate”                   | MUST NOT grant child-task authority or change implementation ownership; MUST keep work with its explicit owner or report unresolved topology.                          |
| Existing suitable child owner                   | MUST reuse that owner; MUST NOT create a replacement or patch its branch.                                                                                              |
| Required app-task surface or authority absent   | MUST keep selected child-owned work pending and report the exact limitation; MUST NOT silently take it over.                                                           |
| Consumer needs a prerequisite artifact          | MUST sequence the producer before the consumer and name the needed artifact or decision.                                                                               |
| Ready lanes list an overlapping path            | Path overlap alone MUST NOT serialize dispatch; MUST keep dependency-ready lanes concurrent within available capacity.                                                 |
| Exclusive resource is in use                    | MUST reserve that concrete resource for its use period and defer only the work that depends on it.                                                                     |
| Actual incompatible edits are found             | Owners MUST coordinate; the owning branch MUST resolve the conflict before merge, preserve both intended changes, and rerun affected checks.                           |
| Child work is accepted                          | The parent MUST retain cleanup ownership and follow the canonical accepted-child procedure; acceptance MUST NOT transfer cleanup to the child.                         |

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
