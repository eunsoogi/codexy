# Token-Efficient Coordination

## Purpose

MUST keep Codexy coordination small without weakening evidence. MUST use this
skill when a thread is recovering from compaction, receiving child terminal
state, routing review feedback, or preparing a handoff that might otherwise
repeat large unchanged artifacts.

This skill summarizes proof and byte comparisons; token billing, wall-time, and
no-change parent-turn savings remain unmeasured.

## Same-Turn Execution Default

- MUST carry finite-phase criteria, terminal submission route, and the reserved
  supervised slot in the initial assignment by reference to their canonical
  contracts. A ready independent Worker MUST execute its authorized
  implementation, verification, selected review, and handoff without routine
  parent approvals. Only required parent environment validation needs a short
  setup phase with an explicit release condition. Shared-file/resource
  conflicts, actual authority, safety decisions, required review, and slot
  capacity MUST remain gates.
- While authorized work or observation remains actionable, preserve the full
  assignment, active goal owner, and next action. Same-turn waiting applies only
  while the host keeps its supported route; it does not imply runtime suspension
  of a parent turn.
- Same-owner re-entry MUST follow the actual
  [goal-lifecycle](../../goal-lifecycle/SKILL.md) readback and preserve the
  latest per-target cursors under [parent supervision](parent-supervision.md).
  Advance only on actual results; input at re-entry MUST NOT imply wait
  cancellation or resumption.
- Outside a route-required quiet wait, MUST provide concise progress commentary
  at meaningful checkpoints during active work; commentary does not end the turn
  or prove completion. Phase outcome and final handoff MUST follow the canonical
  [proof](../../proof-driven-completion/SKILL.md),
  [goal lifecycle](../../goal-lifecycle/SKILL.md), and
  [reporting](goal-transition-reporting.md) contracts.
- This default MUST NOT create busy polling, endless loops, repeat unchanged
  work, artificial token use, exceed host limits, or extend work beyond
  authorized scope.

Task-to-task prompts, progress and callback messages, handoffs, and tool prompt
fields MUST be treated as user-visible and MUST follow the shared
[plain-language message rule](plain-language-user-replies.md). Use IDs and
structured fields to remove repetition while keeping surrounding prose and
protected technical text intact.

Live Sentinel observation MUST remain read-only and event-driven. Generic task
and ledger polling MUST NOT replace cursor-based `wait_threads` observation of
Worker targets; see [parent supervision](parent-supervision.md). The same
reviewer MUST remain pending until its natural terminal result and MUST NOT be
polled, messaged, interrupted, replaced, or duplicated. Use
[review-lifecycle](review-lifecycle.md) for reviewer state, and the canonical
[goal lifecycle](../../goal-lifecycle/SKILL.md),
[proof](../../proof-driven-completion/SKILL.md), and
[reporting](goal-transition-reporting.md) contracts for phase state and handoff.

## Proof State To Retain

MUST retain issue/PR, branch/worktree, owner boundary and Worker task id,
head/base SHAs, checks, unresolved review ids and outdated state, verification
commands/results, and merge readiness or explicit wait/stop condition.

Some lane gates may legitimately be absent, especially in issue-only, pre-PR,
Orchestrator-owned, or pre-review loops. MUST record those slots explicitly as
not-created or not-applicable states with a short reason instead of inventing
evidence or stalling the lane. MUST use those states only when the gate
genuinely does not apply or has not been created yet. For gates that MUST exist
for the current lane, refresh existing gates directly instead of inferring them
from older context.

## Event-driven delta

MUST use this flow after compaction and before handoff:

1. **Inventory once**: MUST keep one compact ledger line per active lane with
   `issue`, `PR`, `branch`, `head`, `owner`, and `state`.
2. **Accept qualifying events only**: the Orchestrator MUST use event-driven
   `wait_threads` for assigned Worker targets and preserve each target's latest
   cursor. Result sufficiency, bounded evidence reads, report routing, host
   limits, quiet waits, cancellation, and fallback MUST follow
   [parent-supervision.md](parent-supervision.md). Native-reviewer terminal
   delivery is a separate route and MUST NOT be confused with Worker
   observation. Unchanged active-goal reads, routine pre/post/continuation
   receipts, liveness-only goal-status messages, normal progress, intermediate
   successful tests, resolved command mistakes, commits, and queued CI MUST
   remain internal; they MUST NOT wake the Orchestrator. For absence
   classifications, apply [observation-evidence.md](observation-evidence.md).
   Workers MUST send compact deltas for terminal Worker state, their
   fatal/gate/final callbacks, PR creation, a required external check-state
   change, actionable review feedback, or review-thread resolution. Selected
   reviewers MUST send their verdicts.
3. **Validate stable event identity**: every event MUST use a deterministic
   `<kind>|<lane>|<subject>` identity. The ledger MUST reject a repeated
   identity before it changes counters or next actions.
4. **Project before returning**: MUST filter large thread/tool results inside
   `functions.exec` before returning them to the model. For `read_thread`, MUST
   retain current thread/turn status and the exact terminal/current
   `agentMessage`, exact source ids/links, and typed safety envelope from
   [`context-tiers.md`](context-tiers.md). MUST preserve unknowns as unknown.
   `turnLimit` and per-item character caps do not bound nested item count, so
   MUST filter items/fields independently. MUST omit reasoning, repeated
   progress, unrelated tool bodies, arguments, and stale turns. MUST keep
   unresolved finding ids/status; load details only when needed. MUST NOT return
   the raw response or drop unresolved proof to meet a size budget.
5. **Fail once**: a failed Orchestrator message MUST emit exactly one terminal
   unavailable report. It MUST include its event identity and MUST NOT retry the
   Orchestrator message.
6. **Carry one next action**: each lane MUST end with exactly one current
   action.
7. **Use the runtime wait contract**: monitor identity, scheduled observations,
   and suppression of unchanged continuation turns MUST follow
   [runtime-heartbeats](runtime-heartbeats.md).

Native goal status, outcome judgment, and worker-to-parent delivery MUST follow
[goal-lifecycle](../../goal-lifecycle/SKILL.md),
[proof-driven-completion](../../proof-driven-completion/SKILL.md), and
[goal-transition-reporting](goal-transition-reporting.md), respectively. This
coordination guide MUST NOT define a second phase-completion or handoff rule.

## Event Delta Shape

MUST use this compact shape for each lane:

```text
#<issue> / PR #<pr> / <branch>
event id: <kind>|<lane>|<subject>
event kind: terminal-child | sentinel | pr-created | new-head | check-state | review-feedback | review-clean | unavailable
owner: Worker task <id> | worktree <path>
head: <sha> | base: <sha>
delta: <one changed fact>
required gates: checks=<state>; threads=<state>; child=<state>
active obligations: <only current unresolved work>
stale/demoted: <old heads, resolved threads, superseded comments>
next action: <one action>
```

When no qualifying event arrived, MUST NOT wake the implementation lane. The
Orchestrator MAY retain its compact ledger without re-reading old details.

The event shape keeps evidence compact; its explanatory fields MUST follow the
shared message rule.

## Runtime Heartbeats

MUST use [runtime-heartbeats.md](runtime-heartbeats.md) for registration,
schedules, fingerprints, cleanup, and Worker idle handoffs. A heartbeat MUST NOT
replace a live Sentinel's no-poll route. The owner MUST record the
Orchestrator's active overall goal separately from the heartbeat registration.

For repeat handoffs, copy [the delta-poll template](../templates/delta-poll.md)
and fill only the current slots. MUST keep the template output in the thread or
handoff; MUST NOT attach old logs or unchanged review bodies unless a current
gate points to them.

## Compaction Budget

After compaction, MUST refresh the single active-lane inventory: latest SHAs,
unresolved review ids, child ownership/stop condition, current-gate proof, and
relevant tool exposure mismatches. MUST reuse prior commands only while they
prove that gate. MUST NOT reload unchanged references, old review/command
output, resolved feedback, or closed lanes unless referenced by a current gate.

## Handoff Discipline

For a Worker handoff or Orchestrator status, include:

- `remember`: durable facts needed for the next gate,
- `refresh`: facts that MUST be re-polled before action,
- `forget`: resolved, outdated, superseded, or irrelevant details; MUST NOT
  drive work unless a fresh poll makes them current again,
- `next`: one action with the owner.

The parent MUST handle actionable results from this compact current-head state
first, reading linked artifacts or bounded transcript details only for missing,
truncated, contradictory, or unbound evidence. MUST NOT require prolonged
transcript re-reading as a routine phase-approval step.

## Stop Conditions

MUST stop and refresh rather than summarizing when a qualifying event reports:

- the head SHA changed,
- a check moved from pending to pass/fail;
- a review thread changed resolved or outdated state;
- child ownership is unclear; or
- the next action would merge, resolve a review thread, or claim readiness.

These actions MUST require current authoritative evidence, not a cached summary.
