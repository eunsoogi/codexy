---
name: goal-state-recovery
description: Use when real goal tools (`create_goal`, `get_goal`, or `update_goal`) are used, or when resuming a task controlled by a goal state; do not load it for work that does not use goal tooling.
---

# Goal State Recovery

## Purpose

Use this skill only for a real goal-tool operation or a resume of a task whose
execution is controlled by a goal state. The host goal tools remain
authoritative. This skill recovers a stale `blocked` control-plane record so the
existing owner can resume; it does not implement goal state or replace the owner
thread, branch, or worktree.

## Required first transition

Before any edit, command, verification, GitHub mutation, delegation, or other
task work, call `get_goal` and use its current result:

1. If the result is exactly `null` or has exactly `status=complete`, create the
   new finite goal normally.
2. If the result is `active`, compare its objective with the requested work.
   Continue only when it is the exact active objective. Otherwise stop and
   obtain an explicit lifecycle disposition; do not overwrite the active goal.
3. If the result is `blocked`, do not work under that goal. Preserve the
   existing owner, branch, worktree, and task context while performing only the
   recovery transition below.
4. For an error, `unknown`, `missing`, malformed result, or any other unexpected
   state, preserve the exact readback and stop without task work.

For delegated children, preserve the existing `$orchestration` pre-delivery,
terminal-handoff, and post-result receipts around the goal calls. These receipts
are control-plane reporting only; they do not release ownership, authorize task
work, or alter the recovery sequence. A delivery failure stops the transition
without retrying it.

For a `blocked` result, perform this exact sequence:

1. Call `update_goal(status="complete")` only to terminate the stale blocked
   execution record.
2. State that this is an administrative control-plane unblock. It is not
   evidence that the issue, PR, implementation, proof, merge, release, or
   external gate is complete.
3. Read back the cleared goal state. Continue only when the authoritative
   readback is exactly `goal=null` or exactly `status=complete`. If it is
   `active`, `blocked`, an error, `unknown`, `missing`, malformed, or any other
   unexpected state, preserve the exact receipt and stop.
4. Create a new finite goal for the same authorized work, read back that the new
   goal is `active`, and create or refresh the current plan.
5. Resume task work only after the new goal is confirmed `active`.

If administrative completion or fresh-goal creation fails, perform no task work
without a newly confirmed `active` goal. Preserve the exact tool result, retain
the owner lane, and stop. This means no edit, command, verification, delegation,
or GitHub mutation after either failure. A blocked-goal recovery does not
authorize abandoning or duplicating the owner task.

## Refusal-only fallback

The administrative `update_goal(status="complete")` route is the default. If and
only if the host explicitly refuses it because the unfinished blocked objective
cannot be marked `complete`, use this bounded control-plane fallback:

1. Preserve the exact refusal and send the required parent transition receipt.
   Do not retry the refused call. A timeout, permission failure, transport
   failure, or ambiguous error is not this fallback; preserve it and stop.
2. Fork the blocked task with `fork_thread` in the same directory. Do not create
   a competing worktree or branch. If the fork does not return one new task
   owner, preserve the exact result and stop.
3. After the fork succeeds, archive the original blocked task with
   `set_thread_archived(archived=true)`. If archiving fails, neither task may
   perform work; preserve the original owner reservation and stop for an
   explicit lifecycle disposition.
4. In the forked task, call `get_goal` before any work and confirm the exact
   result is `null`. If it is `blocked`, `active`, `complete`, or an error,
   preserve the exact result and stop; do not fork or retry again.
5. Only after the original task is archived and the fork's `null` result is
   confirmed, create the fresh finite goal, read back `active`, and create or
   refresh the current plan. Resume work only then.

This fallback preserves singular ownership of the existing branch, worktree, and
task context; it is not issue, PR, implementation, proof, merge, release, or
external-gate completion. Parent-delivery and post-result receipts around the
fork, archive, and goal calls remain control-plane evidence and do not authorize
task work before the new goal is active.

## Completion boundary

The `complete` transition in the blocked recovery sequence is administrative
control-plane state only. `$proof-driven-completion` still owns every ordinary
completion claim. In particular, the recovery transition cannot prove or
substitute for issue, PR, implementation, verification, review, CI, merge,
release, publication, or external-gate evidence. A normal finite-goal completion
remains subject to the ordinary proof and handoff rules.

No repository or external mutation may occur between observing `blocked` and
confirming the new goal is `active`. Use the existing `$orchestration`
transition receipts for delegated parent delivery and exact post-result
readback. Do not add a parser, validator, hook, workflow, schema, runtime
service, or compatibility wrapper for this behavior.

## Verification

This is an instruction-only skill, so do not manufacture prose RED/GREEN. Where
isolated host tasks are available, exercise the real goal surface for:

- `null` and exact `status=complete` results creating a fresh active goal;
- an exact `active` objective continuing without replacement, and a different
  objective stopping for lifecycle disposition;
- `blocked` administrative completion followed by cleared-state readback and a
  fresh active goal;
- an explicit completion refusal followed by the same-directory fork, original
  task archival, forked `goal=null` readback, and a fresh active goal;
- fork or archival failure preserving the exact error and doing no work;
- administrative completion failure preserving the exact error and doing no
  work;
- fresh-goal creation failure preserving the exact error and doing no work.

The #712 refusal shape is corrected only when the host either accepts the
administrative control-plane transition without treating it as an issue or
product-completion claim, or completes the refusal-only fork fallback with the
same-directory and ownership readbacks above. Real goal-tool readback is the
evidence; local fixtures, parser logic, and mock-only assertions are not
substitutes.
