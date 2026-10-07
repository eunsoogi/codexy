---
name: goal-lifecycle
description: Use before any explicitly authorized Orchestrator or implementation Worker task, including read-only diagnosis, when using goal tools or resuming a task controlled by a goal, or for same-directory fork recovery of a blocked task. The native read-only Watcher exception remains separate. MUST NOT infer execution or goal authority from ordinary questions, ambiguous discussion, or unassigned suggestions.
---

# Goal Lifecycle

## Purpose

Codex MUST use this skill before explicitly assigned Orchestrator or
implementation Worker task work, including read-only diagnosis, and for every
goal-tool operation or resume of a task controlled by a goal. Codex MUST load it
before task-specific work, not only after choosing a goal-tool operation. The
native read-only Watcher exception below remains separate and MUST NOT create a
separate goal. Codex MUST treat the host goal tools as authoritative. The
lifecycle governs fresh goal startup, active continuation, and authorized
recovery from blocked goals. It MUST NOT change host goal state, issue
ownership, branch, or worktree to simulate completion. Recovery changes only the
active task thread through a same-directory fork and archive.

## Delegated assignment authorization

A concrete delegated task assignment is explicit authorization for the assigned
child to create one finite execution goal for that same work. The assignment
MUST name the authorized objective and success criteria; an issue-sized handoff
that also supplies scope, verification, and a stop condition satisfies this
contract. The child MUST NOT require a second instruction containing the word
`goal` or another opt-in phrase.

The parent handoff MUST state the assigned objective once as
`Assignment objective:`. The child MUST use that exact value for its finite goal
and MUST NOT broaden it or treat it as a second authorization or user opt-in.

For an Orchestrator/Worker arrangement, the Orchestrator MUST establish or
continue its exact overall goal before its task work and before Worker dispatch.
Before dispatch, it MUST give each Worker one exact `Assignment objective:`
line, bounded scope, success criteria, verification, stop condition, and an
explicit direction to complete the Worker's own first transition before
task-specific work. A parent goal MUST NOT stand in for a Worker's task-scoped
goal.

The finite goal MUST match the assignment and MUST NOT broaden scope, invent
work, replace external proof, or override the authoritative lifecycle state. A
parent instruction that requires non-trivial child implementation while saying
that available goal tools are unauthorized is contradictory and MUST be
rejected; the child MUST NOT follow that prohibition. Ambiguous conversation,
incidental discussion, and unassigned suggestions are not delegated task
assignments and MUST NOT create a goal.

### Native watcher exception

When the Orchestrator summons the packaged `codexy-watcher` role through the
native subagent facility for bounded observation and Watcher MCP reporting, the
Orchestrator retains the overall goal and all judgement, correction, and
acceptance authority. The Watcher MUST NOT create, transfer, complete, or block
a separate overall goal. The Watcher MCP transports the bounded assignment and
reports; it does not grant the Watcher goal ownership. The delegated-assignment
rule above continues to apply to implementation Workers, not to this native
Watcher role. The Watcher keeps that assignment in one native turn and MUST NOT
return after one report, one Worker completion, or an empty timeout while an
assigned target remains nonterminal; it returns only for full assignment
completion, explicit cancellation, or a verified host limitation.

Only the assigned Watcher MAY call `wait_threads` for its assigned Worker or
task targets. The Orchestrator MUST await `watcher_wait` and MUST NOT directly
wait on those targets. Fallback, unavailable, and host-transition branches MUST
recover the supported Watcher route rather than authorize direct parent polling.
An implementation Worker or child MUST NOT open, wait on, report to, cancel, or
reuse a parent-owned Watcher session or token.

During that assignment, ordinary Worker progress, completion, findings, and
attention reports MUST go to the exact Watcher task supplied by the Orchestrator
through the host's supported task-message route. The Watcher deduplicates
unchanged reports and relays only meaningful changes or required decisions.
Blocked-goal recovery and terminal handoff receipts remain direct-parent
control-plane messages. Initial goal registration and active readback stay in
the owning task and MUST NOT be sent as separate parent reports. A verified
unavailable route or concrete emergency permits one marked direct-parent
fallback, not routine duplicate reporting.

## Required first transition

Before any edit, command, verification, GitHub mutation, delegation, or other
task work, the task MUST call `get_goal` and MUST use its current result:

1. If the result is exactly `null`, is a host response envelope whose top-level
   `goal` is exactly `null`, or has exactly `status=complete`, the task MUST
   create the new finite goal for the authorized assignment.
2. If the result is `active`, the task MUST compare its objective with the
   requested work. The task MUST continue only when it is the exact active
   objective. Otherwise the task MUST stop and MUST obtain an explicit lifecycle
   disposition; the task MUST NOT overwrite the active goal.
3. If the result is `blocked`, the task MUST NOT work under that goal. The
   blocked state alone does not authorize a resume. After explicit authorization
   to continue the same objective, the task MUST preserve the lane owner,
   branch, worktree, and task context while performing only the recovery
   transition below.
4. For an error, `unknown`, `missing`, malformed result, or any other unexpected
   state, the task MUST preserve the exact readback and MUST stop before task
   work.

For initial registration, the owning task MUST use the native `get_goal`,
`create_goal` when allowed, and active `get_goal` readback results directly. It
MUST complete that sequence before implementation or dispatch and MUST NOT send
pre-delivery or post-result parent reports for those calls. Existing
blocked-goal recovery and terminal-handoff reporting rules continue to apply to
their respective transitions.

## Authorized blocked-goal recovery

The task MUST NOT call `update_goal` to mark an unfinished blocked objective
complete or otherwise clear it for recovery. The task MUST NOT resume from a
blocked state without explicit authorization for the same objective. Until the
fork's goal is active, the task MUST NOT perform issue implementation or
verification, edit the repository, change the branch or worktree, or mutate
GitHub. It MAY perform only the read-only preflight and recovery operations
listed below.

For an authorized resume, the owner MUST preserve the exact blocked-goal
readback, source task id and parent, objective, current directory, branch,
worktree, HEAD, dirty/index state, and source model and reasoning effort. The
owner MUST record requested settings from the source task assignment or creation
record separately from any effective settings the host exposes. For each
setting, the owner MUST use the effective source value when observable and
otherwise its exact requested value. If either continuation value is unavailable
from both sources, the task MUST stop before forking and report the limitation;
it MUST NOT use sender or role defaults.

1. The task MUST call `fork_thread` for the blocked task in the same directory.
   It MUST NOT create a competing branch, worktree, or task. If the call fails
   or does not return exactly one new task owner, it MUST preserve the exact
   result and stop without retrying.
2. When an authenticated source parent and terminal handoff receipt are bound to
   this task, the task MUST send exactly one receipt before source archival. The
   receipt MUST identify the fork, preserved lane state, and requested and
   effective model/effort evidence. If no source parent or receipt route is
   bound, the task MUST NOT invent a recipient, notification, or approval gate.
   If a required receipt cannot be delivered, the task MUST preserve the failure
   evidence, keep the blocked source as owner, and MUST NOT archive or continue
   the fork.
3. The task MUST archive the original blocked task with
   `set_thread_archived(archived=true)`. If this fails, it MUST NOT send a
   continuation to the fork or allow either task to work; it MUST preserve the
   original owner reservation and exact error, then stop for parent disposition.
4. The task MUST read back that the original is archived and the fork is the
   sole active owner in the same directory before continuing. If this cannot be
   confirmed, it MUST preserve the exact readback and stop without authorizing
   task work.
5. Only after that readback, the continuation owner MUST send the fork a
   `send_message_to_thread` prompt with the source `model` and `thinking` values
   selected above. The continuation owner is the authenticated source parent
   when one exists, otherwise the current authorized task owner. The prompt MUST
   carry the same objective and require native `get_goal` before task work. If
   the tool cannot accept both values, it MUST stop without omitting either or
   relying on defaults. If delivery fails or is ambiguous, the owner MUST
   preserve the exact result, keep the fork reserved without task work, and stop
   without retrying or forking again.

In the fork, the first task action MUST be `get_goal`. An exact `null`, a
response envelope whose top-level `goal` is exactly `null`, or exact
`status=complete` permits creation of a fresh finite goal for the same
authorized objective; the fork MUST create it and read back `active`. An
`active` result permits continuation only when its objective exactly matches;
the fork MUST NOT replace it. If goal creation or its active readback fails or
differs, the fork MUST preserve the exact result and stop without task work.
Plan handling remains governed by `$orchestration` and `$planning`; this
recovery MUST NOT introduce a plan-file requirement by itself. A `blocked`
result, different active objective, error, `unknown`, `missing`, malformed
result, or other unexpected state MUST be preserved exactly and MUST stop. The
task MUST NOT fork again or retry the recovery sequence.

Requested model/effort fields prove only what the continuation requested. If the
host does not expose effective settings, the task MUST record them as
unavailable and MUST NOT claim effective model preservation was observed.
Fork/archive and goal readbacks are control-plane evidence; none proves issue,
PR, implementation, verification, review, CI, merge, release, publication, or
external-gate completion.

## Deciding Whether Work Is Blocked

The host's `update_goal(blocked)` contract remains authoritative; this skill
MUST NOT claim to change it. A repetition threshold, including one accrued by
host-triggered continuations after the same wait, MUST NOT by itself satisfy
blocked criteria. The task MUST also meet every current host criterion,
including the distinct inability to make meaningful progress without user input
or an external state change.

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

The task MUST NOT use a `complete` transition to recover a blocked goal. The
task MUST use `$proof-driven-completion` for every ordinary completion claim. A
normal finite goal MUST NOT become complete until its objective is achieved and
ordinary proof and handoff rules are satisfied.

Between observing `blocked` and confirming the fork's goal is `active`, only the
fork/archive, required parent handoff, and native goal readback steps above are
allowed. The task MUST use the existing `$orchestration` receipt contracts.
Initial `get_goal`, `create_goal`, and active readback stay in the owning fork
without separate registration reports. This instruction-only behavior MUST NOT
add a parser, validator, hook, workflow, schema, runtime service, or
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
- One same-directory fork for an authorized blocked resume, source archival
  before continuation, exactly one active owner, explicit source model/effort in
  the continuation request, and a fresh active goal before work. Effective
  model/effort MUST be reported separately and as unavailable when the host does
  not expose them;
- Fork, archive, owner-readback, continuation, and fresh-goal creation failures
  preserving the exact result and stopping task work without another fork.

Real app task, archive, and goal readbacks MUST be the evidence for recovery;
the task MUST NOT use local fixtures, parser logic, or mock-only assertions as
substitutes. Recovery evidence MUST show the source remained blocked through
archival; it MUST NOT depend on an `update_goal(complete)` attempt or result.
