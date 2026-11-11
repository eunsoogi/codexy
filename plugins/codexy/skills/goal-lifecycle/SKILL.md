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
simulate completion. Fork recovery changes only the active task thread through a
same-directory fork and archive; a supported same-task resume retains it.

## Goal-start contract

A delegated assignment MUST distinguish the overall issue outcome from the
child's currently executable phase. The parent MUST state the phase objective
once as `Assignment objective:`, with bounded scope, success criteria,
verification, handoff, and a stop condition. Preparation, implementation, or
verification MAY be separate phases only when explicitly assigned; the child
MUST NOT invent a narrower phase to clear an unfinished goal. The phase criteria
MUST cover the assigned deliverable, required proof, and any review or execution
result required for that phase. The child MUST keep the phase unfinished while
any such criterion is unmet; native goal status MUST follow the applicable
lifecycle rules below. Parent merge, parent acceptance or acknowledgement, and
future instructions MUST NOT be phase completion criteria.

The child MUST use that exact phase objective for its finite goal. When the host
requires an explicit user or system request, neither the parent assignment nor
scoped orchestration authority for child tasks or messages supplies it. An
applicable explicit user request covering the parent goal and multiple Worker
goals MUST be reused for every covered goal; the task MUST NOT require a
separate unchanged confirmation for each Worker or another opt-in merely because
a covered goal is assigned its own phase. The parent MUST include the request's
source and scope in the child context or provide an authenticated reference when
the original request is not visible to the child. A later user stop or
revocation MUST override that authority for its stated scope. Plan-only or
question-only requests and unassigned suggestions MUST NOT create execution
goals. The task MUST preserve the native `get_goal` readback and report
genuinely missing authority before task work. If the host cannot support the
required operation, it MUST identify that host boundary accurately; it MUST NOT
infer permission or call a forbidden `create_goal`.

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
sequence. For fork recovery, before post-archive acceptance, the task MUST NOT
perform issue implementation or ordinary issue verification. It MAY perform only
the bounded read-only preflight, necessary preservation of owned work, and other
recovery operations specified in
[executable-checkout recovery](references/executable-checkout-recovery.md).

Active-goal Worker elevation is a separate same-issue route. It preserves the
matching active source goal under a cooperative no-write instruction and follows
the
[Worker elevation transition](../orchestration/references/worker-elevation/transition.md)
only when separately authorized. It MUST NOT set the source goal to `blocked` or
`complete` to obtain a fork or avoid automatic continuation. Blocked recovery
MUST preserve its source-pair rules and MUST NOT be repurposed for model
elevation.

Before any fork or source archival, the owner MUST follow the linked
same-directory recovery procedure. The owner MUST verify the native fork result
and run a lightweight read-only process at the exact CWD with matching Git
checkout identity. `list_artifacts` and separate host lifetime guarantees MAY
help diagnose an actual checkout failure; `attach_worktree` MAY repair an
observed association failure when its ownership rules permit. These steps MUST
NOT be blanket prerequisites for a confirmed same-directory checkout. An empty,
PR-only, unsupported, or failed artifact listing MUST NOT invalidate a
successful exact-CWD process or imply that another task lacks a checkout.

The owner MUST archive the source chat after the continuation's actual checkout
and process checks succeed, then read back the archive, sole owner, goal, and
process at the same checkout. If the continuation still has the same blocked
goal after the source is archived, the owner MUST keep it in recovery-only mode
and continue the same procedure from that current owner. Issue work MUST resume
only after the continuation has the exact authorized active objective and a
successful process at the preserved checkout. If an actual CWD process or
checkout comparison fails, the owner MUST repair that observed failure when a
supported operation permits it; otherwise it MUST stop issue work and report the
exact host result.

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

Ordinary CI, review, dependency, resource, and other producer waits are
nonterminal while a required phase criterion remains unmet. Retryable errors and
unfinished but actionable authorized work are also nonterminal. For an ordinary
producer wait, while work remains, the task MUST retain ownership of the exact
active objective, use the existing event-driven or same-turn wait route, and
continue available work on re-entry. The task MUST NOT repeat model turns for
unchanged waits or to reach a threshold.

When parent action or an external event can advance an unmet phase, the task
MUST keep its actual goal state and ownership. Use
[goal-transition-reporting](../orchestration/references/goal-transition-reporting.md)
for the readable table report and handoff, and
[runtime-heartbeats](../orchestration/references/runtime-heartbeats.md) for the
supported wake route. Neither a wait nor unavailable monitoring changes the
blocked-state criteria above. The task MUST NOT ask the user to clear the goal
to resume the same assignment or fabricate a `complete`, `blocked`, or `paused`
transition to make a wait disappear. This ordinary wait route MUST NOT override
a direct user stop, a pause explicitly requested by the user, or a
host-permitted blocked transition under the lifecycle rules above.

This ordinary wait route applies only while an assigned phase criterion remains
unmet. Parent-owned work after a completed-phase handoff MUST NOT keep the
child's goal active.

## Completion boundary

The task MAY call `update_goal(complete)` only after
[$proof-driven-completion](../proof-driven-completion/SKILL.md) establishes that
the assigned phase's criteria and required proof are satisfied and the required
handoff follows
[goal-transition-reporting](../orchestration/references/goal-transition-reporting.md).
The host result controls the actual goal state. A completed goal ends only that
finite phase; it MUST NOT imply issue closure, PR merge, release, or parent
acceptance. After submitting the required handoff, the child MUST NOT keep this
goal open for parent merge, acceptance or acknowledgement, or future
instructions. The parent MUST continue its separate overall goal through the
supported next action in
[goal-transition-reporting](../orchestration/references/goal-transition-reporting.md).
Follow-up work MUST use the same owner's actual lifecycle readback; a different
unfinished objective MUST stop, and blocked recovery MUST follow the ordered
same-scope procedure.

Before fork recovery's post-archive acceptance, the task MAY perform only the
recovery preparation and native operations in the linked procedure, including
necessary change preservation and checkout-association checks. The fork's first
`get_goal` and any permitted fresh-goal creation stay inside that continuation
and MUST NOT generate registration reports. The task MUST use the existing
`$orchestration` receipt contracts. For fork recovery, issue work MUST resume
only after source archival, one continuation owner, the exact authorized active
objective, and a successful process at the preserved checkout are confirmed. A
supported same-task resume instead follows the separate acceptance checks in the
linked procedure. This instruction-only behavior MUST NOT add a parser,
validator, hook, workflow, schema, runtime service, or compatibility wrapper.

## Verification

This is an instruction-only skill. The task MUST NOT manufacture prose
RED/GREEN. Reuse authorized real recovery readbacks when they cover the
behavior; the task MUST NOT create a redundant fork or competing owner solely to
manufacture evidence. An explicitly authorized verification scenario MAY use a
fork when needed. If no authorized real recovery exists, the task MUST report
the live app behavior as unavailable/unverified. When an authorized real fork
recovery is available, the task MUST verify:

- Exact `null`, a `goal=null` response envelope, and exact `status=complete`
  results creating a fresh active goal;
- An exact active objective continuing without replacement, and a different
  active objective stopping for lifecycle disposition;
- Existing assignment authority for the same objective and scope, while honoring
  user stops, revoked authority, scope changes, host denials, and unresolved
  material user-input gates that remain in force until their specific answers;
- Necessary owned-work preservation; the actual same-directory fork, exact CWD
  and Git identity; successful lightweight processes before and after source
  archival; archived-source readback; and one continuation owner. An artifact
  listing or lifetime guarantee MUST NOT be required when these checks pass. The
  owner MUST continue recovery from the blocked fork under the original
  assignment. Requested and effective settings MUST remain separate; unavailable
  values MUST remain unavailable;
- Fork, archive, owner-readback, continuation, and fresh-goal creation failures
  preserving the exact result and stopping issue work until the actual owner and
  checkout permit recovery to continue from the current owner.

Real app task and goal readbacks MUST be the evidence for recovery; the task
MUST NOT use local fixtures, parser logic, or mock-only assertions as
substitutes. Fork recovery evidence MUST also include archive readback and show
the source remained blocked through archival; it MUST NOT depend on an
`update_goal(complete)` attempt or result.
