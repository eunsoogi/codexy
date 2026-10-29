# Runtime Heartbeats

For ordinary Worker completion or attention waits, the Orchestrator MUST use
event-driven `wait_threads` with each target's latest cursor. Reserve heartbeat
scheduling for genuinely scheduled monitoring or when `wait_threads` is
unavailable.

## Same-Turn Wait Handling

- While authorized implementation, verification, coordination, or an assigned
  supported observation remains actionable, MUST preserve the full assignment,
  active goal owner, and next uncompleted action. A same-turn wait applies only
  while the host keeps the existing supported route available; this instruction
  MUST NOT imply runtime-owned parent suspension.
- An empty, unchanged, or timed-out `wait_threads` result, or
  a tool output yield, MUST NOT by itself complete the assignment, trigger an
  unchanged status or reasoning turn, or narrow the objective. When the
  observation obligation remains, continue only through its selected supported
  route using the latest returned cursor and actual host limits. MUST NOT
  replace cursor waits with repeated polling. If an outer `functions.exec` cell
  is pending, resume it rather than issue another wait.
- A shorter wait or its empty result alone MUST NOT alert. Any shorter wait MUST
  state a reason and MUST NOT become repeated polling. MUST report a
  host/fallback limit only when no supported route permits continuation and
  parent action is required.
- A new user input may end `wait_threads` early; commentary does not wake that
  wait. Handle input at actual host re-entry, but input or caller re-entry alone
  MUST NOT prove that a pending wait was cancelled or resumed. Keep host
  interruption distinct from user input; neither proves a Worker event. If the
  host cannot continue the existing cell or deliver input, record that limit; do
  not claim runtime-owned suspension or create an unverified loop.
- An output-yield cadence alone MUST NOT be treated as shortening the semantic
  wait; applicable higher-priority instructions and actual host limits still
  govern every blocking call. MUST NOT bypass cursor waits.
- Outside a route-required quiet wait, MUST provide concise progress commentary
  at meaningful checkpoints during active work; commentary does not end the turn
  or prove completion. A short wait result alone MUST NOT trigger a final status
  or idle handoff. The
  [goal and terminal lifecycle](#goal-and-terminal-lifecycle) below still
  permits a finite idle handoff when its actual conditions are met.
- This default MUST NOT create rapid polling, busy loops, duplicate unchanged
  work, artificial token use, work beyond the authorized scope, or waits beyond
  host limits. MUST follow [parent supervision](parent-supervision.md) for
  direct wait ownership and per-target cursors.

For ordinary Worker observation, after a host transition or
`No handler
registered` failure, the owner MUST treat the mismatch as
host-transition exposure evidence, perform one fresh thread-tool discovery and
one host-aware `wait_threads` retry before any fallback, MUST NOT use unbounded
`read_thread`, and any bounded metadata fallback MUST consume the current
Orchestrator-stage budget and record only returned size/token metadata.

For ordinary Orchestrator-owned Worker observation with a callable
`wait_threads` handler, the Orchestrator MUST use the cursor-based wait while
the assignment remains. A native reviewer's terminal delivery is its own surface
and MUST NOT be confused with Worker observation. The host's automatic
continuation behavior is not implied and MUST be reported if observed. If a
slingshot-host turn still returns
`No handler registered` after the one fresh discovery and one host-aware retry,
the owner MUST emit exactly one unavailable evidence receipt and require
desktop-origin root re-entry; it MUST NOT repeat the wait call, schedule a
heartbeat relay, use `read_thread`, or use `handoff_thread` for recovery. The
slingshot recovery route is not an unavailable-wait fallback eligible for
heartbeat registration; it ends in desktop-origin root re-entry.

## Eligibility And Discovery

When genuinely scheduled monitoring or an unavailable `wait_threads` route other
than slingshot recovery will outlive the current turn, the owning Orchestrator
or Worker MUST search the callable tool surface for `automation_update` before
declaring persistent monitoring unavailable. A callable heartbeat surface is
`automation_update` with a thread-targeted `kind=heartbeat`. For such genuinely
scheduled monitoring or unavailable-wait fallback other than slingshot recovery,
the owner MUST register a heartbeat instead of repeated model continuations or
ending without a wakeup path. This route requires explicit scheduled-monitoring
authorization and MUST NOT be used to preserve an unconditional Orchestrator
turn. The heartbeat MUST use a thread-targeted `kind=heartbeat`. Its schedule
MUST be bounded to the external gate's expected window. For the current thread,
creation MUST use
`destination="thread"` rather than inventing or copying a target-thread id.
Creation MUST use a heartbeat name, prompt, bounded schedule, active status, and
heartbeat kind; the owner MUST retain the returned automation id. It MUST view
the heartbeat by that id before relying on the monitor. The heartbeat automation
id, target thread, bounded schedule, and last observed state fingerprint are the
runtime-issued identity for this route. A heartbeat automation route MUST NOT
require a persistent exec/session id or same-process resume; those fields
identify a separate process-backed monitor route.

## Registration Evidence And Prompt

The owner MUST record the automation id, target thread, bounded schedule, stable
observed-state identity, eligible material events, and terminal delete/disable
action. The observed-state identity MUST be a deterministic fingerprint of the
gate inputs, such as a PR head plus check/review/thread state. Eligible material
events are a terminal child result, a Sentinel verdict, a new HEAD, a GitHub
check-state change, actionable review feedback, review-thread resolution, or an
explicit user/Orchestrator message. The prompt MUST suppress unchanged
observations and MUST wake the owner only for a material gate change or an
explicit user/Orchestrator message.

A live Sentinel, pending Worker, queued CI, pending connector review,
Orchestrator authorization, dependency integration, or resource slot is a
nonterminal producer. The owner MUST preserve ownership through a nonterminal
wait handoff while immediately executable work remains, or through an idle-wait
handoff only after the assigned finite phase's criteria, proof, and handoff are
satisfied. A wait alone MUST NOT satisfy those criteria. The owner MUST use its
event route rather than declaring an execution impasse. An unavailable wake
route does not authorize a blocked goal; only an unanswered material user
decision or missing user information can do so.

## Goal And Terminal Lifecycle

The following lifecycle applies to ordinary Workers and other owners.

A successfully registered heartbeat is runtime-owned waiting; it does not
authorize goal creation or prove phase completion. Ordinary Worker phases MUST
follow [goal-lifecycle](../../goal-lifecycle/SKILL.md): the assignment's exact
phase objective and criteria are distinct from the overall issue outcome.

- During preparation, implementation, or verification, the owner MUST retain its
  matching active goal while any assigned criterion or proof remains unmet,
  including required CI or review. It MUST record `goal state=active` and
  `goal transition=none` and use the supported wait/return route when needed.
- An external wait or no immediately executable work MUST NOT turn an unmet
  phase into a complete one. The owner MUST preserve the actual goal state and
  remaining proof; it MUST NOT narrow a broader active goal, invent a completed
  subphase, or use false `complete`, `paused`, or `blocked` to clear a wait.
- Once all stated phase criteria, proof, and required handoff delivery are
  satisfied, the owner MUST complete only that finite phase and leave the task
  idle. It MUST NOT wait for a parent reply, merge, or future instructions to
  complete the phase. The idle-wait handoff in `goal-transition-reporting.md`
  MUST preserve ownership, branch/worktree, artifacts, external gates, and the
  parent-owned next action. No heartbeat is required for an explicit parent
  wake.
- Phase completion MUST NOT claim overall issue completion, PR merge, release,
  transfer, or parent acceptance. The Orchestrator's overall goal remains
  separate from each Worker's phase.
- For follow-up fixes or another qualifying event, the same owner MUST first
  read the actual lifecycle state. It MUST resume a matching active objective;
  on null/complete it MUST create the authorized fresh phase and read back
  `active` before edits, proof, or review responses. Creation MUST remain
  subject to host authority, including any explicit user/system goal-request
  rule. A different unfinished objective MUST stop for supported disposition;
  `blocked` MUST follow the existing goal-lifecycle recovery sequence.

The awakened owner MUST consume the event in the same turn and record the
resulting lifecycle state in the compact lane delta. When no further observation
is required, it MUST delete the heartbeat by id or disable it with a paused
automation status and its full update fields, and record the cleanup. Automation
status MUST NOT be confused with a goal's lifecycle state.

## Unavailable And Sentinel Boundaries

If heartbeat automation is not callable, the owner MUST record the exact
discovery/exposure evidence and use a bounded fallback wake route without
fabricating a monitor identity or repeating unchanged continuation turns; it
MUST mark automation id, schedule, and lifecycle as not-created. The owner MUST
NOT fold a live packaged Sentinel into heartbeat observation: Sentinel
observation remains read-only, event-driven, and subject to its
no-poll/no-message boundary.
