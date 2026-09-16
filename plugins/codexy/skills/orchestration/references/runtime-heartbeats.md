# Runtime Heartbeats

For ordinary non-Watcher Worker completion or attention waits, the owner MUST
use event-driven `wait_threads` with each target's latest cursor as the default.
The owner MUST reserve heartbeat scheduling for genuinely scheduled monitoring
or when `wait_threads` is unavailable. A Watcher is a bounded native subagent
summoned by the Orchestrator, not a separate app task, heartbeat, or automatic
scheduler.

For an ordinary non-Watcher owner, after a host transition or
`No handler
registered` failure, the owner MUST treat the mismatch as
host-transition exposure evidence, perform one fresh thread-tool discovery and
one host-aware `wait_threads` retry before any fallback, MUST NOT use unbounded
`read_thread`, and any bounded metadata fallback MUST consume the current
Orchestrator-stage budget and record only returned size/token metadata. On a
native Watcher route, the assigned Watcher owns the wait/retry for assigned
targets; the Orchestrator MUST use `watcher_wait` and MUST NOT directly retry or
poll those targets.

For an ordinary non-Watcher owner with a callable `wait_threads` handler, the
owner MUST use the cursor-based wait for an assigned observation obligation
while that obligation remains. A native reviewer's terminal delivery is its own
non-Watcher surface and MUST NOT be confused with Worker observation. Native
Watcher roles, report routing, host limits, quiet waits,
interruption/cancellation, deduplication, and return conditions are defined in
[parent-supervision.md](parent-supervision.md); MUST read that reference before
using a native Watcher. Only the assigned Watcher MAY wait on assigned Worker
targets, and the Orchestrator MUST await `watcher_wait` without directly waiting
or polling them. For ordinary owners outside the Watcher route, the caller
controls whether to return after the bounded wait; the host's automatic
continuation behavior is not implied by this contract and MUST be reported if
observed. If a slingshot-host turn still returns `No handler registered` after
the one fresh discovery and one host-aware retry, the owner MUST emit exactly
one unavailable evidence receipt and require desktop-origin root re-entry; it
MUST NOT repeat the wait call, schedule a heartbeat relay, use `read_thread`, or
use `handoff_thread` for recovery. The slingshot recovery route is not an
unavailable-wait fallback eligible for heartbeat registration; it ends in
desktop-origin root re-entry.

## Native Watcher boundary

Native Watcher creation, assignment, report routing, cursor waits, host and
transport limits, quiet waiting, interruption/cancellation, deduplication, and
Orchestrator/Worker ownership are defined in
[parent-supervision.md](parent-supervision.md). MUST read that reference before
using a native Watcher. A heartbeat MUST NOT replace that route, and the Watcher
MUST remain observation-only. The Orchestrator MUST NOT directly poll Worker
targets assigned to a native Watcher.

## Eligibility And Discovery

When genuinely scheduled monitoring or an unavailable `wait_threads` route other
than slingshot recovery will outlive the current turn, the owning Orchestrator
or Worker MUST search the callable tool surface for `automation_update` before
declaring persistent monitoring unavailable. A callable heartbeat surface is
`automation_update` with a thread-targeted `kind=heartbeat`. For such genuinely
scheduled monitoring or unavailable-wait fallback other than slingshot recovery,
the owner MUST register a heartbeat instead of repeated model continuations or
ending without a wakeup path. This route requires explicit scheduled-monitoring
authorization; it MUST NOT be recreated merely to replace an app Watcher or
preserve an unconditional Orchestrator turn. The heartbeat MUST use a
thread-targeted `kind=heartbeat`. The heartbeat schedule MUST be bounded to the
external gate's expected window. For the current thread, creation MUST use
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
explicit user/Orchestrator message. Watcher reports use the core MCP path
defined in [parent-supervision.md](parent-supervision.md) and are not heartbeat
events or a reason to create a heartbeat.

A live Sentinel, pending Worker, queued CI, pending connector review,
Orchestrator authorization, dependency integration, or resource slot is a
nonterminal producer. The owner MUST preserve ownership through a nonterminal
wait handoff while immediately executable work remains, or through an idle-wait
handoff after that finite work is complete, and MUST use its event route rather
than declaring an execution impasse. An unavailable wake route does not
authorize a blocked goal; only an unanswered material user decision or missing
user information can do so.

## Goal And Terminal Lifecycle

The following lifecycle applies to ordinary Workers and other owners; the
Orchestrator remains the owner of the overall goal while a Watcher subagent is
active. A Watcher report MUST NOT create, replace, or transfer that goal.

A successfully registered heartbeat is runtime-owned waiting. The owner MUST
retain its active goal and plan only while an immediately executable in-scope
obligation remains, record `goal state=active` and `goal transition=none`, and
return control without completing or blocking the goal. When no immediately
executable obligation remains and only an external event or explicit
Orchestrator wake can advance work, the owner MUST send the idle-wait handoff
defined in `goal-transition-reporting.md`, complete the finite execution phase,
and leave the task idle without claiming issue, implementation, transfer, or
release completion. This finite phase is distinct from the Orchestrator's
overall goal and a Watcher's bounded observation assignment; neither a phase
completion nor a Watcher report proves release completion. A qualifying event
MUST create a fresh short-lived execution goal and current plan before any edit,
proof, review response, publication, or merge work. The awakened owner MUST
first read the actual lifecycle state and MUST continue a matching active
objective; if the state is null or complete, it MAY create the fresh goal and
MUST read back `active`. A different unfinished objective requires a supported
lifecycle disposition and MUST NOT be overwritten. An observed `blocked` state
remains governed by the existing `goal-lifecycle` recovery authority; this
reference MUST NOT replace or restate that recovery sequence. The awakened owner
MUST consume the event in the same turn and MUST delete or disable the heartbeat
when no further observation is required. It MUST record the resulting lifecycle
state in the compact lane delta. When cleanup is needed, the owner MUST delete
the heartbeat by id or disable it with a paused status and the heartbeat's full
update fields; it MUST record which terminal action occurred.

## Unavailable And Sentinel Boundaries

If heartbeat automation is not callable, the owner MUST record the exact
discovery/exposure evidence and use a bounded fallback wake route without
fabricating a monitor identity or repeating unchanged continuation turns; it
MUST mark automation id, schedule, and lifecycle as not-created. The owner MUST
NOT fold a live packaged Sentinel into heartbeat observation: Sentinel
observation remains read-only, event-driven, and subject to its
no-poll/no-message boundary.
