# Token-Efficient Coordination

## Purpose

MUST keep Codexy coordination small without weakening evidence. MUST use this
skill when a thread is recovering from compaction, receiving child terminal
state, routing review feedback, or preparing a handoff that might otherwise
repeat large unchanged artifacts.

This skill summarizes current proof and byte comparisons without changing which
obligations apply; token billing and wall-time savings remain unmeasured.

Task-to-task prompts, progress and callback messages, handoffs, and tool prompt
fields MUST be treated as user-visible and MUST follow the shared
[plain-language message rule](plain-language-user-replies.md). Use IDs and
structured fields to remove repetition while keeping surrounding prose and
protected technical text intact.

Live Sentinel observation MUST be read-only and event-driven. Generic task and
ledger polling remains permitted only for the owner's own non-Watcher target,
including a native reviewer's terminal delivery. The Orchestrator MUST NOT use
it for Worker host observation or targets assigned to a native Watcher. The
Watcher MUST NOT directly observe, read, wait on, or poll a native Sentinel; its
observation targets MUST remain limited to its assigned Workers and their scoped
artifact or tool-call channel. Both the Worker owner and the root Orchestrator
MUST NOT message, interrupt, replace, duplicate, follow up with, or poll a live
Sentinel. When only a native-reviewer event remains pending, the Worker MUST NOT
start unchanged model-continuation turns or poll the reviewer. Once the current
finite execution phase is satisfied, the Worker MUST use the existing finite
idle-wait handoff; that finite phase transition MUST be reported separately and
MUST NOT be treated as completion of an unmet issue, release, or long-lived
goal. If the current finite phase objective is genuinely unmet, the Worker MUST
retain its honest goal state and return control through the supported wait or
terminal-delivery path; it MUST NOT force-complete the unmet goal or edit
goal-lifecycle state merely to escape the wait. A bounded wait with no event is
a non-terminal `PENDING` observation, and an independently observed live
reviewer is `RUNNING`; neither observation is a reviewer verdict or
fallback-eligible. The owning lane MUST retain the same reviewer and wait for
its natural terminal result. A live Sentinel MUST report its own terminal
`PASS`, `BLOCK`, or `UNOBSERVABLE` result naturally. A native reviewer's
terminal delivery is its own non-Watcher surface and MUST NOT be treated as
Worker host observation.

## Proof State To Retain

MUST retain these state slots for an active lane:

- issue and PR numbers,
- branch and worktree path,
- owner boundary and Worker task id,
- current head SHA and base SHA,
- current check state,
- unresolved review thread ids and whether they are outdated,
- verification commands and results,
- merge readiness or explicit wait/stop condition.

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
2. **Accept qualifying events only**: for ordinary non-Watcher waits, the owner
   MUST use event-driven `wait_threads` with each target's latest cursor. This
   route covers a task's own non-Watcher target, including a native reviewer's
   terminal delivery, and MUST NOT authorize the Orchestrator to observe Worker
   targets assigned to a native Watcher. Native Watcher waits, report routing,
   host limits, quiet waits, interruption/cancellation, and fallback MUST follow
   [parent-supervision.md](parent-supervision.md); only the assigned Watcher MAY
   wait on its Worker/task targets, and the Orchestrator MUST await
   `watcher_wait` without direct polling or unbounded `read_thread`. Workers
   MUST NOT open, wait on, report to, cancel, or reuse an Orchestrator-owned
   Watcher session or token. The Watcher MUST NOT create or own the Orchestrator
   goal. A Watcher callback is material only when its event identity is new and
   Orchestrator action is required. Unchanged active-goal reads, routine
   pre/post/continuation receipts, liveness-only goal-status messages, normal
   progress, intermediate successful tests, resolved command mistakes, commits,
   and queued CI MUST remain internal; they MUST NOT wake the Orchestrator. For
   absence classifications, apply
   [observation-evidence.md](observation-evidence.md). Workers MUST send compact
   deltas for terminal Worker state, their fatal/gate/final callbacks, PR
   creation, a required external check-state change, actionable review feedback,
   or review-thread resolution. Watchers MUST send their own compact deltas for
   observation-channel failure or actionable drift, and selected reviewers MUST
   send their verdicts. A Watcher drift event is qualifying only when its report
   is grounded in a changed artifact, diff, or relevant actual tool call,
   identifies the conflicting current scope, ownership, or user constraint
   without a repair directive, and requires an Orchestrator decision; relayed
   Worker or Orchestrator findings MUST remain distinct from Watcher-first
   detection.
3. **Validate stable event identity**: every event MUST use a deterministic
   `<kind>|<lane>|<subject>` identity. The ledger MUST reject a repeated
   identity before it changes counters or next actions.
4. **Promote ids, not prose**: MUST keep exact ids and links. MUST NOT transfer
   a full conversation, full tool body, or full agent-tree listing. Direct reads
   and command output MUST remain bounded.
5. **Fail once**: a failed Orchestrator message MUST emit exactly one terminal
   unavailable report. It MUST include its event identity and MUST NOT retry the
   Orchestrator message.
6. **Carry one next action**: each lane MUST end with exactly one current
   action.
7. **Require runtime polling evidence**: polling/monitoring MUST be reserved for
   an observation bound to one complete runtime-issued monitor identity. A
   heartbeat route MUST bind the observation to its heartbeat automation id,
   target thread, bounded schedule, and last observed state fingerprint or event
   identity. The heartbeat route MUST NOT require a persistent exec/session
   identifier or same-process resume. A separate process-backed monitor MUST
   bind the observation to a persistent runtime monitor or wait session id, a
   scheduled next-observation time or deadline, the last observed state
   fingerprint or event identity, and same-process resume. Distinct
   model/assistant turn ids, tool-driven re-entry, goal continuation, or agent
   invocation without either complete runtime-issued identity are continuation
   turns, not polling; unchanged continuation turns MUST NOT reschedule
   themselves or emit another unchanged turn.
8. **Suppress unchanged continuation turns**: when an authorized Worker-local
   monitor observes no qualifying event and the stable event identity, head,
   checks, review state, and next action are unchanged, it MUST keep the monitor
   scheduled but MUST NOT emit a status message or start another model turn. The
   next scheduled read-only observation MAY run at its bounded interval. A new
   model turn may start only when that monitor observes a qualifying event, or
   when an explicit Orchestrator/user message arrives. This rule MUST NOT
   terminate or cancel the underlying wait/monitor session.

Before a Worker stops, archives, yields ownership, or calls
`update_goal(complete)` or `update_goal(blocked)`, it MUST send exactly one
terminal handoff delta to the source Orchestrator. `update_goal(blocked)`
additionally requires the typed unanswered user-decision gate; token pressure,
repeated continuations, unchanged fingerprints, external producers, and
coordination waits MUST NOT authorize a blocked goal. That delta MUST include
the stable event identity, issue/PR, child task id, branch/worktree, exact HEAD
and dirty/index state, last completed proof, current external gate, preserved
artifacts or reservation, and one Orchestrator-owned next action. The Worker
MUST confirm task-surface delivery before the stop/archive or goal transition. A
failed delivery MUST emit one unavailable receipt and MUST NOT retry or
transition.

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

Heartbeat registration, bounded schedules, state fingerprints, cleanup, ordinary
Worker lifecycle, and idle-wait handoffs are defined in
[runtime-heartbeats.md](runtime-heartbeats.md). MUST read that reference before
using a heartbeat or handling an ordinary Worker idle-wait handoff. A heartbeat
MUST NOT replace the native Watcher route or the live Sentinel's event-driven
no-poll boundary. The Orchestrator retains the active overall goal while the
Watcher observes and MUST record those goal and bounded-assignment states
separately.

For repeat handoffs, copy [the delta-poll template](../templates/delta-poll.md)
and fill only the current slots. MUST keep the template output in the thread or
handoff; MUST NOT attach old logs or unchanged review bodies unless a current
gate points to them.

## Compaction Budget

After compaction, rebuild only the working set:

- active lanes and their latest known SHAs,
- unresolved review thread ids,
- child ownership and stop condition,
- commands already run only when their result still proves a current gate,
- known tool exposure mismatches that affect the next action.

MUST NOT reload old full review bodies, full command output, resolved feedback,
or closed lanes unless a current gate references them.

## Handoff Discipline

For a Worker handoff or Orchestrator status, include:

- `remember`: durable facts needed for the next gate,
- `refresh`: facts that MUST be re-polled before action,
- `forget`: resolved, outdated, superseded, or irrelevant details,
- `next`: one action with the owner.

MUST use `forget` for stale context. It means the detail MUST NOT drive active
work unless a fresh poll makes it current again.

## Stop Conditions

MUST stop and refresh rather than summarizing when a qualifying event reports:

- the head SHA changed,
- a check moved from pending to pass/fail;
- a review thread changed resolved or outdated state;
- child ownership is unclear; or
- the next action would merge, resolve a review thread, or claim readiness.

These actions MUST require current authoritative evidence, not a cached summary.
