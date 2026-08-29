---
name: goal-state-recovery
description: Use when real goal tools (`create_goal`, `get_goal`, or `update_goal`) are used, or when resuming a task controlled by a goal state; MUST NOT load it for work that does not use goal tooling.
---

# Goal State Recovery

## Purpose

Codex MUST use this skill only for a real goal-tool operation or a resume of a
task whose execution is controlled by a goal state. Codex MUST treat the host
goal tools as authoritative. This skill recovers a stale `blocked` control-plane
record so the existing owner can resume. It MUST NOT implement goal state or
replace the owner thread, branch, or worktree.

## Required first transition

Before any edit, command, verification, GitHub mutation, delegation, or other
task work, the task MUST call `get_goal` and MUST use its current result:

1. If the result is exactly `null`, is a host response envelope whose top-level
   `goal` is exactly `null`, or has exactly `status=complete`, the task MUST
   create the new finite goal normally.
2. If the result is `active`, the task MUST compare its objective with the
   requested work. The task MUST continue only when it is the exact active
   objective. Otherwise the task MUST stop and MUST obtain an explicit lifecycle
   disposition; the task MUST NOT overwrite the active goal.
3. If the result is `blocked`, the task MUST NOT work under that goal. The task
   MUST preserve the existing owner, branch, worktree, and task context while
   performing only the recovery transition below.
4. For an error, `unknown`, `missing`, malformed result, or any other unexpected
   state, the task MUST preserve the exact readback and MUST stop before task
   work.

For delegated children, the task MUST preserve the existing `$orchestration`
pre-delivery, terminal-handoff, and post-result receipts around the goal calls.
These receipts are control-plane reporting only; the task MUST NOT treat them as
releasing ownership, authorizing task work, or altering the recovery sequence. A
delivery failure MUST stop the transition, and the task MUST NOT retry it.

For a `blocked` result, the task MUST perform this exact sequence:

1. The task MUST call `update_goal(status="complete")` only to terminate the
   stale blocked execution record.
2. The task MUST state that this is an administrative control-plane unblock.
   The task MUST NOT treat it as evidence that the issue, PR, implementation,
   proof, merge, release, or external gate is complete.
3. The task MUST read back the cleared goal state. The task MUST continue only
   when the authoritative readback is exactly `goal=null` or exactly
   `status=complete`. If it is `active`, `blocked`, an error, `unknown`,
   `missing`, malformed, or any other unexpected state, the task MUST preserve
   the exact receipt and MUST stop.
4. The task MUST create a new finite goal for the same authorized work, MUST read
   back that the new goal is `active`, and MUST create or refresh the current
   plan.
5. The task MUST resume work only after the new goal is confirmed `active`.

If administrative completion or fresh-goal creation fails, the task MUST NOT
perform task work unless a newly confirmed `active` goal exists. The task MUST
preserve the exact tool result, MUST retain the owner lane, and MUST stop. After
either failure, the task MUST NOT edit, command, verify, delegate, or mutate
GitHub. A blocked-goal recovery MUST NOT authorize abandoning or duplicating the
owner task.

## Refusal-only fallback

The task MUST treat the administrative `update_goal(status="complete")` route
as the default. The task MUST use this bounded control-plane fallback if and
only if the host explicitly refuses that route because the unfinished blocked
objective cannot be marked `complete`:

1. The task MUST preserve the exact refusal and MUST send the required parent
   transition receipt. The task MUST NOT retry the refused call. A timeout,
   permission failure, transport failure, or ambiguous error MUST NOT be treated
   as this fallback; the task MUST preserve it and MUST stop.
2. The task MUST fork the blocked task with `fork_thread` in the same directory.
   The task MUST NOT create a competing worktree or branch. If the fork does not
   return one new task owner, the task MUST preserve the exact result and MUST
   stop.
3. After the fork succeeds, the task MUST archive the original blocked task
   with `set_thread_archived(archived=true)`. If archiving fails, the task MUST
   NOT allow either task to perform work; the task MUST preserve the original
   owner reservation and MUST stop for an explicit lifecycle disposition.
4. In the forked task, the task MUST call `get_goal` before any work and MUST
   confirm the exact result is `null`. If it is `blocked`, `active`, `complete`,
   or an error, the task MUST preserve the exact result and MUST stop; the task
   MUST NOT fork or retry again.
5. Only after the original task is archived and the fork's `null` result is
   confirmed, the task MUST create the fresh finite goal, MUST read back
   `active`, and MUST create or refresh the current plan. The task MUST resume
   work only then.

This fallback MUST preserve singular ownership of the existing branch, worktree,
and task context. The task MUST NOT treat it as issue, PR, implementation,
proof, merge, release, or external-gate completion. Parent-delivery and
post-result receipts around the fork, archive, and goal calls MUST remain
control-plane evidence; they MUST NOT authorize task work before the new goal is
active.

## Completion boundary

The task MUST treat the `complete` transition in the blocked recovery sequence
as administrative control-plane state only. The task MUST use
`$proof-driven-completion` for every ordinary completion claim. In particular,
the task MUST NOT use the recovery transition to prove or substitute for issue,
PR, implementation, verification, review, CI, merge, release, publication, or
external-gate evidence. A normal finite-goal completion MUST remain subject to
the ordinary proof and handoff rules.

Between observing `blocked` and confirming the new goal is `active`, the task
MUST NOT perform repository or external mutation. The task MUST use the existing
`$orchestration` transition receipts for delegated parent delivery and exact
post-result readback. The task MUST NOT add a parser, validator, hook, workflow,
schema, runtime service, or compatibility wrapper for this behavior.

## Verification

This is an instruction-only skill. The task MUST NOT manufacture prose RED/GREEN.
Where isolated host tasks are available, the task MUST exercise the real goal
surface for:

- The task MUST verify `null`, a `goal=null` response envelope, and exact
  `status=complete` results creating a fresh active goal;
- The task MUST verify that an exact `active` objective continues; the task MUST
  NOT replace it, and a different objective MUST stop for lifecycle disposition;
- The task MUST verify `blocked` administrative completion followed by
  cleared-state readback and a fresh active goal;
- The task MUST verify an explicit completion refusal followed by the
  same-directory fork, original task archival, forked `goal=null` readback, and
  a fresh active goal;
- The task MUST verify fork or archival failure preserving the exact error and
  doing no work;
- The task MUST verify administrative completion failure preserving the exact
  error and doing no work;
- The task MUST verify fresh-goal creation failure preserving the exact error and
  doing no work.

The task MUST treat the #712 refusal shape as corrected only when the host either
accepts the administrative control-plane transition and the task MUST NOT treat
it as an issue or product-completion claim, or completes the refusal-only fork
fallback with the same-directory and ownership readbacks above. Real goal-tool
readback MUST be the evidence; the task MUST NOT use local fixtures, parser
logic, or mock-only assertions as substitutes.
