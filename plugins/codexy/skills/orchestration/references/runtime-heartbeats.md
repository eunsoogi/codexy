# Runtime Heartbeats

The owner MUST use event-driven `wait_threads` with each target's latest cursor
as the default for ordinary child completion or attention waits. The owner MUST
reserve heartbeat scheduling for genuinely scheduled monitoring or when
`wait_threads` is unavailable. A Watcher is a bounded native subagent summoned
by the Orchestrator, not a separate app task, heartbeat, or automatic scheduler.

After a host transition or `No handler registered` failure, the owner MUST treat
the mismatch as host-transition exposure evidence, perform one fresh thread-tool
discovery and one host-aware `wait_threads` retry before any fallback, MUST NOT
use unbounded `read_thread`, and any bounded metadata fallback MUST consume the
current Orchestrator-stage budget and record only returned size/token metadata.

While a desktop-origin root turn has a callable `wait_threads` handler, the
owner MUST use the cursor-based wait for an assigned observation obligation
while that obligation remains. For a native Watcher, this wait belongs to one
long-running subagent turn: after each wait it MUST inspect the actual Worker
result, report a material event when warranted, and continue waiting while any
assigned target remains nonterminal. An unchanged cursor, bounded timeout, one
report, or one Worker completion is nonterminal and MUST NOT end that Watcher
turn. A Watcher may return only after the full assignment is terminal, the user
or Orchestrator explicitly cancels it, or a verified host limitation prevents
continuation. For ordinary owners outside the Watcher route, the caller controls
whether to return after the bounded wait; the host's automatic continuation
behavior is not implied by this contract and MUST be reported if observed. If a
slingshot-host turn still returns `No handler registered` after the one fresh
discovery and one host-aware retry, the owner MUST emit exactly one unavailable
evidence receipt and require desktop-origin root re-entry; it MUST NOT repeat
the wait call, schedule a heartbeat relay, use `read_thread`, or use
`handoff_thread` for recovery. The slingshot recovery route is not an
unavailable-wait fallback eligible for heartbeat registration; it ends in
desktop-origin root re-entry.

## Native Watcher boundary

The Orchestrator MUST summon an explicitly authorized Watcher through the
callable native-subagent API (`spawn_agent` or its versioned multi-agent
equivalent), with exact Worker/task targets. The Watcher observes real Worker
callbacks and task state, and MUST report action-required drift or an
unavailable channel through `watcher_report`. It MUST remain read-only: it MUST
NOT edit, direct or message a Worker, supply a repair directive, correct,
accept, verify a correction, replace, or recruit. A Watcher report is a signal,
not acceptance, and repeated unchanged observations MUST be suppressed.
Unchanged active-goal reads, routine pre/post/continuation receipts, and
liveness-only goal-status messages MUST remain internal. Only an actual
lifecycle transition, unresolved drift or failure requiring Orchestrator action,
missing terminal delivery, or a ready external gate may be reported.

The Orchestrator MUST keep the overall goal active and owned by itself while the
Watcher subagent observes. It MAY use `wait_watcher` with the parent token and
cursor for bounded waiting; a user input or host cancellation MUST release that
wait immediately only when the host propagates it as a same-connection MCP
cancellation. A task message or outer wait termination may leave the native wait
active; report that limitation, use authorized `watcher_cancel`, and open a new
assignment/session for a fresh observation. `watcher_cancel` ends the current
session only. The Watcher has no long-lived release goal. If the host exposes a
finite goal for the subagent, that goal MUST cover only the bounded observation
assignment and MUST NOT be treated as issue or release completion. The Watcher
MUST keep its native turn active after each report and return to its
`wait_threads` loop while an assigned target remains nonterminal; the
Orchestrator may return control while that native turn continues. The Watcher
may return only for full assignment completion, explicit user/Orchestrator
cancellation, or a verified host limitation. The Orchestrator MUST inspect the
relevant Worker/app surface after a material report, decide and instruct the
Worker, and verify the resulting call or diff.

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
