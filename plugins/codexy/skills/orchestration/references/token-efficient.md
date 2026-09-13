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

Live Sentinel observation MUST be read-only and event-driven. Generic child and
ledger polling remains permitted. The Watcher MUST NOT directly observe, read,
wait on, or poll a native Sentinel; its observation targets MUST remain limited
to its assigned Workers and their scoped artifact or tool-call channel. Both the
Worker owner and the root Orchestrator MUST NOT message, interrupt, replace,
duplicate, follow up with, or poll a live Sentinel. When only a native-reviewer
event remains pending, the Worker MUST NOT start unchanged model-continuation
turns or poll the reviewer. Once the current finite execution phase is
satisfied, the Worker MUST use the existing finite idle-wait handoff; that
finite phase transition MUST be reported separately and MUST NOT be treated as
completion of an unmet issue, release, or long-lived goal. If the current finite
phase objective is genuinely unmet, the Worker MUST retain its honest goal state
and return control through the supported wait or terminal-delivery path; it MUST
NOT force-complete the unmet goal or edit goal-lifecycle state merely to escape
the wait. A bounded wait with no event is a non-terminal `PENDING` observation,
and an independently observed live reviewer is `RUNNING`; neither observation is
a reviewer verdict or fallback-eligible. The owning lane MUST retain the same
reviewer and wait for its natural terminal result. A live Sentinel MUST report
its own terminal `PASS`, `BLOCK`, or `UNOBSERVABLE` result naturally.

## Proof State To Retain

MUST retain these state slots for an active lane:

- issue and PR numbers,
- branch and worktree path,
- owner boundary and child thread id,
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
2. **Accept qualifying events only**: for ordinary non-Watcher waits, the root
   Orchestrator MUST use event-driven `wait_threads` with each target's latest
   cursor and batched targets. Unchanged cursors, bounded timeouts, and
   legitimate long commands are nonterminal; they MUST NOT produce repeated
   status messages, full-transcript reads, test reruns, or interruptions.
   Reserve heartbeat scheduling for scheduled monitoring or unavailable
   `wait_threads`. For a native Watcher route, only the assigned Watcher MAY
   call `wait_threads` for its Worker/task targets. The Orchestrator MUST await
   `watcher_wait`; new callers MUST use `watcher_wait` and MUST NOT directly
   wait on those assigned targets. Fallback, unavailable, or host-transition
   branches MUST report the actual limitation, recover the supported Watcher
   route, and MUST NOT authorize direct parent polling or unbounded
   `read_thread`. After an actionable Watcher report, one bounded authoritative
   Worker/app readback for judgement/correction is allowed; it is not an
   observation wait. An implementation Worker or child MUST NOT open, wait on,
   report to, cancel, or reuse a parent-owned Watcher session or token; session
   visibility and parent transcript access are not capability grants. The
   Watcher MUST NOT create or own the Orchestrator goal. A Watcher callback or
   observation is material only when its event identity is new and Orchestrator
   action is required. Unchanged active-goal reads, routine
   pre/post/continuation receipts, liveness-only goal-status messages, normal
   progress, intermediate successful tests, resolved command mistakes, commits,
   and queued CI MUST remain internal; they MUST NOT wake the Orchestrator. For
   absence classifications, apply
   [observation-evidence.md](observation-evidence.md). Workers MUST send compact
   deltas for terminal child state, their fatal/gate/final callbacks, PR
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
8. **Suppress unchanged continuation turns**: when an authorized child-local
   monitor observes no qualifying event and the stable event identity, head,
   checks, review state, and next action are unchanged, it MUST keep the monitor
   scheduled but MUST NOT emit a status message or start another model turn. The
   next scheduled read-only observation MAY run at its bounded interval. A new
   model turn may start only when that monitor observes a qualifying event, or
   when an explicit Orchestrator/user message arrives. This rule MUST NOT
   terminate or cancel the underlying wait/monitor session.

Before a child stops, archives, yields ownership, or calls
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
owner: child thread <id> | worktree <path>
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

For an eligible external gate that outlives the current turn, Orchestrators and
Worker owners MUST follow `$orchestration`'s runtime-heartbeat contract. The
compact lane ledger MUST retain the heartbeat automation id, target thread,
bounded schedule, state fingerprint, material-event set, and delete/disable
state. Heartbeat prompts MUST suppress unchanged observations and MUST wake the
owner only for a material gate change or an explicit user/Orchestrator message.
A stable event identity MUST deduplicate repeated wakeups before the owner
changes its plan. The awakened owner MUST consume a material event in the same
turn and MUST delete or disable its heartbeat when no further observation is
required. A successfully registered heartbeat is runtime-owned waiting. The
heartbeat route is not the ordinary app-thread Watcher: do not create or
recreate a heartbeat as a substitute for the bounded native-subagent observation
route. The Orchestrator retains the active overall goal while the Watcher
observes; neither an idle turn nor a Watcher assignment proves transfer or
completion. Record the Orchestrator goal and bounded Watcher assignment
separately. This does not remove ordinary Worker finite-goal closure or
`blocked` recovery.

For ordinary owners outside the canonical role mapping, the Worker MUST retain
its active goal and plan only while an immediately executable in-scope
obligation remains, record `goal state=active` and `goal transition=none`, and
return control without completing or blocking the goal. When only an external
event or explicit Orchestrator wake remains, the Worker MUST first determine
whether the current finite execution phase is satisfied. If it is satisfied, the
Worker MUST use the idle-wait handoff, complete only that finite phase, and
leave the task idle without claiming the issue, release, implementation, or any
longer-lived goal complete. If the finite phase remains unmet, the Worker MUST
retain the honest goal state required by the existing lifecycle authority and
return control through a supported wait or terminal-delivery path; it MUST NOT
use administrative completion merely to clear the handoff. After a finite phase
is completed and no active execution goal remains, a qualifying event MUST
create a fresh short-lived execution goal and current plan before any edit,
proof, review response, publication, or merge work. If an unmet active finite
goal was retained, a qualifying event MUST resume its existing authorized goal
and plan instead of creating a duplicate. The Orchestrator exemption above
overrides this rule for that role. A live packaged Sentinel remains outside
heartbeat observation and retains its no-poll/no-message boundary.

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
