# Runtime Heartbeats

The owner MUST use event-driven `wait_threads` with each target's latest cursor
as the default for ordinary child completion or attention waits. The owner MUST
reserve heartbeat scheduling for genuinely scheduled monitoring or when
`wait_threads` is unavailable. An app-thread watcher is a separate, explicitly
authorized observation task, not a heartbeat or an automatic scheduler.

After a host transition or `No handler registered` failure, the owner MUST treat
the mismatch as host-transition exposure evidence, perform one fresh thread-tool
discovery and one host-aware `wait_threads` retry before any fallback, MUST NOT
use unbounded `read_thread`, and any bounded metadata fallback MUST consume the
current parent-stage budget and record only returned size/token metadata.

While a desktop-origin root turn has a callable `wait_threads` handler, the
owner MUST use the cursor-based wait for an assigned observation obligation
while that obligation remains. An unchanged cursor or bounded timeout is not a
stall and MUST NOT by itself start another model turn or complete the goal. The
caller controls whether to return after the bounded wait; the host's automatic
continuation behavior is not implied by this contract and MUST be reported if
observed. If a slingshot-host turn still returns
`No handler registered` after the one fresh discovery and one host-aware retry,
the owner MUST emit exactly one unavailable evidence receipt and require
desktop-origin root re-entry; it MUST NOT repeat the wait call, schedule a
heartbeat relay, use `read_thread`, or use `handoff_thread` for recovery. The
slingshot recovery route is not an unavailable-wait fallback eligible for
heartbeat registration; it ends in desktop-origin root re-entry.

## App-thread watcher boundary

An explicitly authorized watcher MUST be an independent Codex app task in the
same saved project as its assigned workers. It observes worker callbacks and
task state, and MUST report material drift or an unavailable channel. It MUST
remain read-only: it MUST NOT edit, correct, accept, replace, or recruit. The actual
creating tool distinguishes this surface from native subagents and packaged
reviewers. A watcher callback is a signal, not acceptance, and repeated
unchanged observations MUST be suppressed.

When the watcher is carrying an exact long-lived release goal, the parent MAY
return control instead of continuing a model turn solely for unchanged waiting.
In watcher-supervised Astra-parent mode, only the watcher carries that goal and
the Astra/medium parent's `get_goal` state MUST remain `null`. The parent MUST
NOT call `create_goal` or recreate any goal for setup, callbacks, correction,
review or merge decisions, or external-event resume; after authorized work it
MUST return control. The watcher goal does not transfer file ownership,
correction authority, final judgment, or issue completion. Codex MUST record
parent and watcher goal states separately. Codex MUST NOT mark an unfinished
parent goal complete, invent a transfer operation, or promise that an idle
parent will wake later. If the host does not expose the requested pause,
transfer, or wake behavior, Codex MUST report it as unsupported. This
parent-only exemption MUST NOT remove ordinary worker finite-goal closure or
`blocked` recovery.

## Eligibility And Discovery

When genuinely scheduled monitoring or an unavailable `wait_threads` route other
than slingshot recovery will outlive the current turn, the owning parent
orchestrator or child MUST search the callable tool surface for
`automation_update` before declaring persistent monitoring unavailable. A
callable heartbeat surface is `automation_update` with a thread-targeted
`kind=heartbeat`. For such genuinely scheduled monitoring or unavailable-wait
fallback other than slingshot recovery, the owner MUST register a heartbeat
instead of repeated model continuations or ending without a wakeup path. This
route requires explicit scheduled-monitoring authorization; it MUST NOT be
recreated merely to replace an app watcher or preserve an unconditional parent
turn. The
heartbeat MUST use a thread-targeted `kind=heartbeat`. The heartbeat schedule
MUST be bounded to the external gate's expected window. For the current thread,
creation MUST use `destination="thread"` rather than inventing or copying a
target-thread id. Creation MUST use a heartbeat name, prompt, bounded schedule,
active status, and heartbeat kind; the owner MUST retain the returned automation
id. It MUST view the heartbeat by that id before relying on the monitor. The
heartbeat automation id, target thread, bounded schedule, and last observed
state fingerprint are the runtime-issued identity for this route. A heartbeat
automation route MUST NOT require a persistent exec/session id or same-process
resume; those fields identify a separate process-backed monitor route.

## Registration Evidence And Prompt

The owner MUST record the automation id, target thread, bounded schedule, stable
observed-state identity, eligible material events, and terminal delete/disable
action. The observed-state identity MUST be a deterministic fingerprint of the
gate inputs, such as a PR head plus check/review/thread state. Eligible material
events are a terminal child result, a Sentinel verdict, a new HEAD, a GitHub
check-state change, actionable review feedback, review-thread resolution, or an
explicit user/parent message. The prompt MUST suppress unchanged observations
and MUST wake the owner only for a material gate change or an explicit
user/parent message.

A live Sentinel, pending child, queued CI, pending connector review, parent
authorization, dependency integration, or resource slot is a nonterminal
producer. The owner MUST preserve ownership through a nonterminal wait handoff
while immediately executable work remains, or through an idle-wait handoff after
that finite work is complete, and MUST use its event route rather than declaring
an execution impasse. An unavailable wake route does not authorize a blocked
goal; only an unanswered material user decision or missing user information can
do so.

## Goal And Terminal Lifecycle

The following lifecycle applies to ordinary workers and other owners that are
not in watcher-supervised Astra-parent mode. For that parent mode, the
parent-supervision exemption above takes precedence: the parent MUST keep
`get_goal=null`, MUST NOT create or recreate a goal on a qualifying event, and
MUST return control after the authorized event work.

A successfully registered heartbeat is runtime-owned waiting. The owner MUST
retain its active goal and plan only while an immediately executable in-scope
obligation remains, record `goal state=active` and `goal transition=none`, and
return control without completing or blocking the goal. When no immediately
executable obligation remains and only an external event or explicit parent wake
can advance work, the owner MUST send the idle-wait handoff defined in
`goal-transition-reporting.md`, complete the finite execution phase, and leave
the task idle without claiming issue, implementation, transfer, or release
completion. This finite phase is distinct from a watcher-held long-lived
release goal; its completion MUST NOT be used as evidence that the watcher goal
or release objective was achieved. A qualifying event MUST create a fresh
short-lived execution goal and current plan before any edit, proof, review
response, publication, or merge work. The awakened owner MUST first read the
actual lifecycle state and MUST continue a matching active objective; if the
state is null or complete, it MAY create the fresh goal and MUST read back
`active`. A different unfinished objective requires a supported lifecycle
disposition and MUST NOT be overwritten. An observed `blocked` state remains
governed by the existing `goal-lifecycle` recovery authority; this reference
MUST NOT replace or restate that recovery sequence. The awakened owner MUST
consume the event in the same turn and MUST delete or disable the heartbeat
when no further observation is required.
It MUST record the resulting lifecycle state in the compact lane delta. When
cleanup is needed, the owner MUST delete the heartbeat by id or disable it with
a paused status and the heartbeat's full update fields; it MUST record which
terminal action occurred.

## Unavailable And Sentinel Boundaries

If heartbeat automation is not callable, the owner MUST record the exact
discovery/exposure evidence and use a bounded fallback wake route without
fabricating a monitor identity or repeating unchanged continuation turns; it
MUST mark automation id, schedule, and lifecycle as not-created. The owner MUST
NOT fold a live packaged Sentinel into heartbeat observation: Sentinel
observation remains read-only, event-driven, and subject to its
no-poll/no-message boundary.
