---
name: goal-lifecycle
description: Use before any explicitly authorized Orchestrator or implementation Worker task, including read-only diagnosis, when using goal tools or resuming a task controlled by a goal, or for same-directory fork recovery of a blocked task. MUST NOT infer execution or goal authority from ordinary questions, ambiguous discussion, or unassigned suggestions.
---

# Goal Lifecycle

## Purpose

Codex MUST use this skill before explicitly assigned Orchestrator or
implementation Worker task work, including read-only diagnosis, and for every
goal-tool operation or resume of a task controlled by a goal. Codex MUST load it
before task-specific work, not only after choosing a goal-tool operation. Codex
MUST treat the host goal tools as authoritative. The lifecycle governs fresh
goal startup, active continuation, and authorized recovery from blocked goals.
It MUST NOT change host goal state, issue ownership, branch, or worktree to
simulate completion. Recovery changes only the active task thread through a
same-directory fork and archive.

## Delegated assignment authorization

A delegated assignment MUST distinguish the overall issue outcome from the
child's currently executable phase. The parent MUST state the phase objective
once as `Assignment objective:`, with bounded scope, success criteria,
verification, handoff, and a stop condition. Preparation, implementation, or
verification MAY be separate phases only when explicitly assigned; the child
MUST NOT invent a narrower phase to clear an unfinished goal. Parent merge,
acknowledgement, or future instructions MUST NOT be phase completion criteria.

The child MUST use that exact phase objective for its finite goal. Assignment
authority MUST NOT override the host's goal-creation rules. When the host
requires an explicit user or system goal request, the task MUST NOT infer that
authority from delegation alone or require a forbidden `create_goal` call. It
MUST preserve the readback and report missing authority before task work. Scoped
orchestration authority covers necessary child tasks and bounded task messages
under [child routing](../orchestration/references/child-routing-policy.md); it
MUST NOT be mistaken for host goal-tool permission. When creation is authorized,
Codex MUST NOT require another opt-in for the same phase. Ordinary questions and
unassigned suggestions MUST NOT create a goal.

For an Orchestrator/Worker arrangement, the Orchestrator MUST establish or
continue its exact overall goal before its task work and before Worker dispatch.
Before dispatch, it MUST give each Worker one exact `Assignment objective:`
line, bounded scope, success criteria, verification, stop condition, and an
explicit direction to complete the Worker's own first transition before
task-specific work. A parent goal MUST NOT stand in for a Worker's task-scoped
goal.

The finite goal MUST NOT broaden scope, invent work, replace external proof, or
override native lifecycle state. An existing broader active objective MUST NOT
be narrowed, replaced, or marked complete to adopt a phase; the child MUST
preserve it and report the mismatch for explicit supported lifecycle
disposition.

## Required first transition

Before any edit, command, verification, GitHub mutation, delegation, or other
task work, the task MUST call `get_goal` and MUST use its current result:

1. If the result is exactly `null`, is a host response envelope whose top-level
   `goal` is exactly `null`, or has exactly `status=complete`, the task MUST
   create the finite phase goal only when the host permits its creation under
   the authorization boundary above; otherwise it MUST stop and report.
2. If the result is `active`, the task MUST compare its objective with the
   requested work. The task MUST continue only when it is the exact active
   objective. Otherwise the task MUST stop and MUST obtain an explicit lifecycle
   disposition; the task MUST NOT overwrite the active goal.
3. If the result is `blocked`, the task MUST NOT perform issue implementation or
   ordinary issue verification under that goal. The original assignment
   continues to authorize recovery of the same objective and scope; blocked
   status alone MUST NOT require new user confirmation. A user stop, revoked or
   missing assignment, changed scope, or actual host permission denial MUST stop
   recovery. Otherwise, the task MAY perform only the bounded recovery
   preparation and native operations in the linked procedure, preserving the
   existing owner, branch, worktree, and task context.
4. For an error, `unknown`, `missing`, malformed result, or any other unexpected
   state, the task MUST preserve the exact readback and MUST stop before task
   work.

For initial registration, the owning task MUST use the native `get_goal`,
`create_goal` when allowed, and active `get_goal` readback results directly. It
MUST complete that sequence before implementation or dispatch and MUST NOT send
pre-delivery or post-result parent reports for those calls. Existing
blocked-goal recovery and terminal-handoff reporting rules continue to apply to
their respective transitions.

On a fresh start where `get_goal` returns `null`, a top-level `goal=null`, or
`status=complete`, this registration sequence MUST be uninterrupted: when
authorized, the owning task MUST call `create_goal` next, then read back the
exact new objective as active before any other task work. It MUST NOT inspect or
report CWD, project, permissions, worktree, checkout, HEAD, index, source, or
issue state, or run a setup or verification command after the initial readback
and before goal creation, or after creation and before the matching active
readback. These read-only setup checks are task work and MUST wait until
active-goal readback. When goal creation is not authorized or fails, the task
MUST stop before those checks. The distinct blocked-goal recovery procedure
below remains unchanged.

## Authorized blocked-goal recovery

The task MUST NOT call `update_goal` to mark an unfinished blocked objective
complete or otherwise clear it for recovery. The original assignment authorizes
recovery only while the same objective and scope remain in force; blocked status
alone MUST NOT trigger a new approval request. A user stop, revoked or missing
assignment, scope change, or actual host permission denial MUST stop the
sequence. Before post-archive acceptance, the task MUST NOT perform issue
implementation or ordinary issue verification. It MAY perform only the bounded
read-only preflight, necessary preservation of owned work, and other recovery
operations specified in
[executable-checkout recovery](references/executable-checkout-recovery.md).

Active-goal Worker elevation is a separate same-issue route. It preserves the
matching active source goal under a cooperative no-write instruction and follows
the
[Worker elevation transition](../orchestration/references/worker-elevation/transition.md)
only when separately authorized. It MUST NOT set the source goal to `blocked` or
`complete` to obtain a fork or avoid automatic continuation. Blocked recovery
MUST preserve its source-pair rules and MUST NOT be repurposed for model
elevation.

Before any fork or source archival, the owner MUST follow the linked procedure.
It owns the fork-goal readback, supported checkout association,
topology-specific lifetime-preservation route, pre-archive checks, source
archival, post-archive acceptance, and failure containment. A task ID, matching
CWD, attachment record, or saved snapshot MUST NOT substitute for a successful
process at the intended checkout after archival. The absence of a direct
goal-resume operation alone MUST NOT be treated as proof that the fork route is
unavailable. If the exact topology's lifetime-preservation route or another
required pre-archive check cannot be established, the owner MUST stop before
archival and report the exact unsupported transition and responsible host
boundary. If post-archive acceptance fails, the owner MUST keep issue work
stopped and report the exact host operation and observed owner state. A result
for one attachment topology MUST NOT be generalized to all checkouts.

## Deciding Whether Work Is Blocked

The host's `update_goal(blocked)` contract remains authoritative; this skill
MUST NOT claim to change it. A repetition threshold, including one accrued by
host-triggered continuations after the same wait, MUST NOT by itself satisfy
blocked criteria. The task MUST also meet every current host criterion,
including the distinct inability to make meaningful progress without user input
or an external state change.

Parent-reported pauses MUST stay parent-sourced; only a direct user message or
authenticated reference establishes intent. Parent direction stays scoped; child
reports or message acceptance MUST NOT prove host state or justify a block.

Ordinary CI, review, dependency, resource, and parent-coordination waits,
retryable errors, and unfinished but actionable authorized work are nonterminal.
While work remains, the task MUST retain ownership of the exact active
objective, use the existing event-driven or same-turn wait route, and continue
available work on re-entry. The task MUST NOT repeat model turns for unchanged
waits or to reach a threshold.

If parent action can advance a dependency, the child MUST send a concise
nonterminal update through the assignment's supported report route naming the
needed parent action and next child action, then preserve ownership through the
supported wake route. When no immediate action remains, the task MUST use the
existing wait/handoff contract and MUST NOT complete a finite phase before its
criteria are met or mark an unfinished assigned goal complete to clear a wait.
The task MUST follow the existing
[goal-transition-reporting](../orchestration/references/goal-transition-reporting.md)
and [runtime-heartbeats](../orchestration/references/runtime-heartbeats.md)
contracts for handoff and finite-phase rules. The task MUST NOT ask the user to
clear the goal to resume the same assignment or use a fabricated `complete`,
`blocked`, or `paused` transition to make a wait disappear.

## Completion boundary

The task MUST use `$proof-driven-completion` and compare the actual active
objective with its stated phase criteria before completion. It MUST complete
only when every criterion, required proof, and handoff is satisfied. A submitted
result alone is insufficient when verification or review remains in that phase.
An external wait, lack of immediate work, or parent silence MUST NOT substitute
for satisfied criteria or justify a false `complete`, `paused`, or `blocked`.

Once the phase criteria and required handoff delivery are satisfied, the child
MUST complete that phase and leave the task idle without waiting for a parent
reply or merge. It MUST preserve issue ownership, branch, worktree, artifacts,
remaining external gates, and the parent-owned next action. Phase completion
MUST NOT imply issue closure, PR merge, release, or parent acceptance. Follow-up
fixes MUST use the same owner and actual lifecycle readback: continue a matching
active objective, or create an authorized fresh phase on null/complete and read
back active before work. A different unfinished objective MUST stop; blocked
recovery MUST follow the ordered same-scope procedure and MUST NOT be reported
as complete when the objective remains unfinished.

Before post-archive acceptance, the task MAY perform only the recovery
preparation and native operations in the linked procedure, including necessary
change preservation and checkout-association checks. The fork's first `get_goal`
and any permitted fresh-goal creation stay inside that continuation and MUST NOT
generate registration reports. The task MUST use the existing `$orchestration`
receipt contracts. Issue work MUST resume only after source archival, one
continuation owner, the exact authorized active objective, and a successful
process at the preserved checkout are confirmed. This instruction-only behavior
MUST NOT add a parser, validator, hook, workflow, schema, runtime service, or
compatibility wrapper.

## Verification

This is an instruction-only skill. The task MUST NOT manufacture prose
RED/GREEN. Reuse authorized real recovery readbacks when they cover the
behavior; the task MUST NOT create a redundant fork or competing owner solely to
manufacture evidence. An explicitly authorized verification scenario MAY use a
fork when needed. If no authorized real recovery exists, the task MUST report
the live app behavior as unavailable/unverified. When an authorized real host
recovery is available, the task MUST verify:

- Exact `null`, a `goal=null` response envelope, and exact `status=complete`
  results creating a fresh active goal;
- An exact active objective continuing without replacement, and a different
  active objective stopping for lifecycle disposition;
- Existing assignment authority for the same objective and scope, while honoring
  user stops, revoked authority, scope changes, host denials, and unresolved
  material user-input gates that remain in force until their specific answers;
- Necessary owned-work preservation, one same-directory fork, the actual fork
  goal-state branch, supported checkout association and lifetime-preservation
  route for the exact topology, pre-archive checks, source archival, and
  post-archive owner, active-objective, and process acceptance before issue
  work. Requested model/effort and host-exposed effective settings MUST remain
  separate, with unexposed values reported as unavailable;
- Fork, archive, owner-readback, continuation, and fresh-goal creation failures
  preserving the exact result and stopping task work without another fork.

Real app task, archive, and goal readbacks MUST be the evidence for recovery;
the task MUST NOT use local fixtures, parser logic, or mock-only assertions as
substitutes. Recovery evidence MUST show the source remained blocked through
archival; it MUST NOT depend on an `update_goal(complete)` attempt or result.
