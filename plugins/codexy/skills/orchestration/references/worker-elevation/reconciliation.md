# Parent-loop Worker elevation reconciliation

## Purpose and boundary

This reference connects the in-flight hard-work decision to the existing
Orchestrator supervision loop. It composes the eligibility assessment in
[`worker-elevation.md`](../worker-elevation.md) with the source checkpoint and
transition procedures in this reference bundle. It MUST preserve the role,
recovery, capacity, observation, receipt, and review contracts those references
already own.

This is an instruction path, not a new executor, serialized state machine,
journal, event store, receipt format, or host capability. Codex MUST update the
existing issue-owner and supervision records. A source-level route, fixture,
successful message request, or green source CI MUST NOT be reported as an
observed native transition. When the host cannot expose a required operation or
readback, the transition MUST remain pending with the source as owner.

## Entry and record ownership

The Orchestrator MAY use this procedure only after the current general Luna/Max
Worker satisfies the elevation decision and the parent separately authorizes the
same-issue transition. Before advancing, it MUST verify that each required
operation and readback is exposed in the current task. This manual procedure
does not imply an automatic entrypoint. If eligibility, the source checkpoint,
target read-only setup, authenticated assignment or parent binding, source-only
retirement, or ownership transfer cannot be established through supported
routes, the parent MUST keep the current owner and report the exact unsupported
step. A missing capability MUST NOT be inferred from a prompt or replaced by a
new API.

The Orchestrator MUST keep one existing issue-owner record authoritative through
the transition. It MUST preserve the issue and PR, original objective, source
and target task identities, original parent, branch/worktree/HEAD, event
identity, observed transition state, recipient and requested/effective settings,
resource reservations, per-task observation cursors, review findings, and open
failure episode. These are existing facts in their existing records; this
reference MUST NOT make a new field set or receipt schema mandatory.

The source remains the sole implementation owner until the target completes
read-only setup, the parent confirms the required bindings, and the source is
safely retired under the transition procedure. The parent MUST keep a prepared
target read-only and MUST NOT assign it implementation work before that point.
The parent owns judgment and acceptance throughout; neither a source report nor
a target acknowledgement transfers ownership.

## Parent-loop sequence

1. **Reconcile eligibility and prerequisites.** The Orchestrator MUST read the
   current source report, native goal, parent instructions, existing owner
   record, and stable event identity, then apply the
   [elevation decision](../worker-elevation.md) to the observed `hard_work`
   evidence. A blocked goal MUST stay on the separate source-pair-preserving
   [blocked-goal recovery](../../../goal-lifecycle/SKILL.md) route. An Advisor
   consultation remains governed by
   [advisor-consultation](../advisor-consultation.md); this clear policy path
   MUST NOT add an Advisor approval step.

   Before preparing a target, the parent MUST confirm that current routing
   compatibility, the source checkpoint, and the separately authorized
   transition apply to the same issue, objective, source, and checkout. If a
   prerequisite or native capability is missing, unsupported, or ambiguous, the
   parent MUST keep the current owner and report the exact gap. It MUST NOT
   dispatch a fork while the source remains blocked or changing, or required
   bindings are unavailable.

2. **Quiesce and checkpoint the source.** Under the continuing cooperative
   no-write instruction, the parent MUST confirm actual completion or
   termination of every identified writer, unchanged source goal and ownership,
   and the checkpoint required by [the source procedure](checkpoint.md). A quiet
   task, acknowledgement, timeout, archive request, or unchanged Git status
   alone MUST NOT stand in for a writer result. The parent MUST recheck the
   checkpoint immediately before the fork. Any new writer or changed source
   state MUST invalidate it.

3. **Reserve actual capacity, then prepare one inert target.** Before forking,
   the parent MUST count the source and target separately whenever both occupy
   execution slots. The ceiling is
   `min(8, any explicitly lower host or resource limit)`. It MUST reserve the
   target slot while retaining the source slot. If capacity is insufficient or
   unobservable, the parent MUST keep elevation pending; a logical issue lane
   does not make two live tasks one slot. It MUST use the supported
   same-directory fork for the exact source task and MUST reconcile an ambiguous
   result by reading the known source, target, and event identities before
   taking any further action. It MUST NOT fork again because a result or
   task-list entry is missing.

   The target MUST use the returned final task identity. The parent MUST confirm
   its lineage, project, checkout association, CWD, branch, HEAD, index, and
   required files against the frozen source checkpoint. The parent MUST NOT
   create a worktree, copy a diff, recreate a branch, or let the target become a
   second writer.

4. **Start the target with a read-only turn.** The parent MUST use a currently
   exposed route to request Terra/Medium before the target's first inference. A
   composer selection or accepted message proves a request or transport only.
   The parent MUST keep requested, accepted, authenticated, effective, and
   actual-execution evidence separate. Codex MUST NOT send a model override to a
   target that remains authenticated as Luna.

   The target's first action MUST be its own native goal readback. It MUST
   retain the exact active assigned objective, or follow the authorized
   goal-startup rules and read back the exact objective as active. If its
   readback is blocked, has a different unfinished objective, is malformed or
   unknown, or errors, the target MUST stop. The target MAY inspect its own
   identity, environment, project, checkout, branch, HEAD, index, required
   files, and supported authenticated message routes. It MUST NOT implement,
   stage, commit, run implementation checks, or start another writer during this
   turn.

   The parent MUST provide its fresh active overall-goal readback. The target
   MUST confirm the authenticated original-parent route; the parent MUST observe
   one target report, send a follow-up to that exact target using its assigned
   pair, and observe the target's read-only receipt of that follow-up. The
   report MUST identify the exact active objective, the target's actual next
   permitted work, and the observed goal, task, model-assignment, parent-route,
   and checkout evidence. The parent MUST verify each claim from the target and
   parent surfaces. Effective-model telemetry MAY remain unavailable only when
   authenticated assignment is otherwise confirmed; a missing or mismatched
   authenticated assignment MUST stop advancement.

5. **Recheck before retirement.** The parent MUST read the known source and
   target independently. It MUST confirm the source's no-write instruction,
   actual writer completion, exact active goal, owner, checkpoint, and current
   parent instructions. It MUST confirm the target identity, active objective,
   authenticated assignment, original-parent route, read-only setup, checkout
   match, and next permitted work. Missing, mismatched, changing, or ambiguous
   evidence stops advancement. The parent MUST preserve both task identities,
   reservations, and the checkout for parent disposition.

6. **Use the existing handoff and source-only retirement.** The source MUST use
   the existing [terminal parent handoff](../goal-transition-reporting.md) and
   transition procedure. It MUST preserve its common fields and event identity
   and add only the transition evidence that procedure already calls for. An
   ambiguous send, callback, or archive result MUST be reconciled against the
   known identities before another send or retirement. The source MUST NOT
   create a second terminal event to repair an unknown callback.

   The parent MUST establish supported source-only retirement and checkout
   preservation before requesting it. Archival alone does not prove that future
   source continuation stopped, that descendants were retired, or that an
   execution slot was freed. The Orchestrator MUST keep the cooperative source
   no-write instruction applicable across later turns. Codex MUST NOT complete,
   block, or pause an unfinished goal to suppress automatic continuation.

7. **Transfer the existing owner record and observe the target.** Only after the
   required source handoff and retirement evidence is reconciled may the parent
   transfer ownership. It MUST update the authenticated target recipient, goal
   association, resource reservation, and observation target together in the
   existing supervision records. It MUST preserve the source pair and transition
   as history. The parent MUST keep the source reservation until a supported
   explicit stop/release and absence of remaining execution are confirmed; it
   MUST NOT free it based on archival, timeout, or omission from a task list.

   The Orchestrator MUST observe ordinary Worker events directly through native
   `wait_threads`. Source and target belong to different task identities and
   MUST retain separate cursors. The Orchestrator MUST initialize the target
   cursor according to native initial-observation rules and MUST NOT pass the
   source cursor as the target's `afterCursor`. It MUST keep other Worker and
   reviewer cursors unchanged. The parent MUST send target follow-ups to the
   authenticated target pair. Existing Sol/Medium recipients MUST continue to
   receive messages through their own assigned pair.

## Duplicate, late, or ambiguous events

Before acting on a repeated request, receipt, timeout, callback, or parent
re-entry, the Orchestrator MUST reconcile the stable event identity and existing
source/target records. It MUST read the known tasks and latest per-task cursors;
an unchanged wait, a partial page, or a missing task-list entry MUST NOT create
a new identity or prove that a task stopped.

- A duplicate request or report for the same transition MUST NOT cause another
  Advisor consultation, target fork, first target turn, terminal handoff, source
  retirement, or slot release.
- A first-delivery timeout or ambiguous transport result MUST be checked on the
  known target and parent surfaces before any resend. An accepted transport is
  not proof of authenticated assignment, parent observation, or approval.
- After transfer, a late source complete, blocked, or idle event is historical
  source evidence. It MUST NOT overwrite target ownership, close the issue,
  release the target reservation, resume the source, or reset reviewer state.
- Parent re-entry after compaction MUST resume from the existing owner/event
  identities and latest cursor for each actual task. It MUST NOT transplant a
  source cursor, recreate a target, or discard the source reservation because a
  record is absent from a summary.
- The parent MUST preserve actionable review findings, repair budgets, and
  failure episodes across the model change and handoff. A model change MUST NOT
  reset them.

When records conflict with native state, or the next owner cannot be
established, the parent MUST preserve both known tasks and reservations, stop
further implementation, and make an explicit disposition. The parent MUST NOT
infer success from a fork ID, target startup, message acceptance, source
archival, or configuration alone.

## Verification boundary

Source checks establish only the instruction and packaged-source behavior they
actually cover. The callable limits and prior observations are recorded in the
[repository's Worker elevation host contract](https://github.com/eunsoogi/codexy/blob/main/docs/worker-elevation-host-contract.md).
They do not establish a live target's effective pair, authenticated parent
binding, source-writer termination, source-only retirement, post-retirement
checkout survival, or successful first implementation step. Those claims require
their own authorized native readbacks; this implementation MUST NOT run a live
fork/archive experiment or upgrade a personal plugin to manufacture them.

Codex MUST keep unsupported native boundaries capability-gated and report them
to the parent. This procedure does not imply an installed or automatic
parent-loop entrypoint; any such claim requires a current authenticated host
readback. It MUST NOT call source fixtures, documentation, or CI native
transition success.
