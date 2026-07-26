# Runtime Heartbeats

The owner MUST use event-driven `wait_threads` with each target's latest cursor as the default for ordinary child completion or attention waits. The owner MUST reserve heartbeat scheduling for genuinely scheduled monitoring or when `wait_threads` is unavailable.

After a host transition or `No handler registered` failure, the owner MUST treat the mismatch as host-transition exposure evidence, perform one fresh thread-tool discovery and one host-aware `wait_threads` retry before any fallback, MUST NOT use unbounded `read_thread`, and any bounded metadata fallback MUST consume the current parent-stage budget and record only returned size/token metadata.

## Eligibility And Discovery

When genuinely scheduled monitoring or an unavailable `wait_threads` route will outlive the current turn, the owning parent orchestrator or child MUST search the callable tool surface for `automation_update` before declaring persistent monitoring unavailable.
A callable heartbeat surface is `automation_update` with a thread-targeted `kind=heartbeat`. For such genuinely scheduled monitoring or unavailable-wait fallback, the owner MUST register a heartbeat instead of repeated model continuations or ending without a wakeup path. The heartbeat MUST use a thread-targeted `kind=heartbeat`.
The heartbeat schedule MUST be bounded to the external gate's expected window.
For the current thread, creation MUST use `destination="thread"` rather than
inventing or copying a target-thread id. Creation MUST use a heartbeat name, prompt,
bounded schedule, active status, and heartbeat kind; the owner MUST retain the
returned automation id. It MUST view the heartbeat by that id before relying on the
monitor.
The heartbeat automation id, target thread, bounded schedule, and last observed
state fingerprint are the runtime-issued identity for this route. A heartbeat
automation route MUST NOT require a persistent exec/session id or same-process
resume; those fields identify a separate process-backed monitor route.

## Registration Evidence And Prompt

The owner MUST record the automation id, target thread, bounded schedule, stable observed-state identity, eligible material events, and terminal delete/disable action.
The observed-state identity MUST be a deterministic fingerprint of the gate inputs,
such as a PR head plus check/review/thread state. Eligible material events are a
terminal child result, a Sentinel verdict, a new HEAD, a GitHub check-state change,
actionable review feedback, review-thread resolution, or an explicit user/parent
message. The prompt MUST suppress unchanged observations and MUST wake the owner only for a material gate change or an explicit user/parent message.

## Goal And Terminal Lifecycle

A successfully registered heartbeat is runtime-owned waiting. The owner MUST end its active goal and plan before waiting. The owner MUST NOT retain or recreate an execution goal solely to preserve a successfully registered heartbeat; it MAY keep a goal only while an implementation obligation remains. A qualifying event MUST start a fresh short-lived execution goal and plan, and the awakened owner MUST consume the event in the same turn and MUST delete or disable the heartbeat when no further observation is required. It MUST record the resulting lifecycle state in the compact lane delta.
When cleanup is needed, the owner MUST delete the heartbeat by id or disable it with
a paused status and the heartbeat's full update fields; it MUST record which terminal
action occurred.

## Unavailable And Sentinel Boundaries

If heartbeat automation is not callable, the owner MUST record the exact discovery/exposure evidence and use a bounded fallback wake route without fabricating a monitor identity or repeating unchanged continuation turns; it MUST mark automation id, schedule, and lifecycle as not-created. The owner MUST NOT fold a live packaged Sentinel into heartbeat observation: Sentinel observation remains read-only, event-driven, and subject to its no-poll/no-message boundary.
