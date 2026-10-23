# External Watcher MCP flow

This reference applies only when the Orchestrator explicitly assigns a Watcher
outside Codexy's packaged specialist catalog. It defines the bounded transport
and supervision flow; it does not register or select a native Watcher role. The
core package no longer registers Watcher MCP or ships the retired launcher and
dedicated bootstrap. The Rust Watcher runtime and interrupt hooks have been
removed from the repository; an assigned host must supply a compatible
implementation and configure its external transport.

- When an external Watcher is assigned, the Orchestrator opens one scoped
  `watcher_open` session for the Orchestrator, Watcher, and exact Worker
  targets. The MCP session transports observations; it does not create or judge
  the Watcher.
- The Watcher MUST follow the bounded criteria in its assignment. Sufficient
  exact target/turn results MUST skip reads/reports when no criterion is due.
  Missing, truncated, contradictory, or unbound evidence requires a bounded read
  of that target. Explicit status MUST queue `health`; required metadata-only
  updates MUST remain non-queued without new triggers or heartbeats.
- The Orchestrator MUST consume one `watcher_wait`: `status`, `sessionId`,
  `nextCursor`, `events`; `health.status`, `watcherState`,
  `lastObservationAtMs`, `lastError`; cancelled-only `cancellationReason`.
  Events MUST contain exactly these seven fields: `eventId`, `sequence`, `kind`,
  `target`, `summary`, `observedAtMs`, and `evidence`. MUST use assignment
  context for removed identity/transport; MUST call `watcher_health` only for
  concrete unresolved diagnostics. Its timestamp is not acceptance proof; MUST
  read Worker/app state only as needed.
- Same-connection `notifications/cancelled` or the retained Watcher
  `PreToolUse`/`Interrupt`/`UserPromptSubmit` hooks releases only that wait when
  the host delivers the event and preserves the durable session.
  `watcher_cancel` instead ends the session and requires a fresh assignment. See
  [Watcher wait cancellation](watcher-wait-cancellation.md) for event matching,
  fail-closed cases, and host-propagation proof boundaries. A cancelled
  queue/cursor is readback only, never continuity; the assignment MUST NOT
  resume.
- The Orchestrator MUST own the exact overall task objective and MUST preserve
  its active goal through Watcher assignment, reports, correction, review, and
  external waits. Assigning a Watcher MUST NOT create a second overall goal or
  move the Orchestrator goal into the Watcher.
- The Watcher subagent MAY receive a finite observation objective, but it MUST
  preserve that bounded scope, MUST NOT overwrite an unrelated active goal, and
  MUST use the existing `goal-lifecycle` recovery authority for any `blocked`
  state. It MUST keep the same native turn active after a material report, one
  Worker completion, or an empty timeout while any assigned target remains
  nonterminal. It MUST return only after the observation assignment is terminal,
  the user or Orchestrator explicitly cancels it, or a verified host limitation
  prevents continuation. It MUST NOT become a separate app task or release-goal
  owner.
- A Watcher report never transfers Worker-file ownership, Orchestrator
  correction authority, final judgement, or issue completion. Codex MUST record
  the Orchestrator's goal and the Watcher's bounded assignment/readback
  separately. A Watcher report is not evidence that the Orchestrator goal was
  paused, cancelled, transferred, or completed.
- Outside this arrangement, the existing finite execution lifecycle permits a
  phase to send its idle-wait handoff, complete that finite phase, and leave the
  task idle when only an external event remains; a qualifying wake creates a
  fresh execution goal and current plan before new work. Codex MUST report that
  phase completion separately from the Orchestrator-owned overall goal and any
  bounded Watcher assignment. It MUST NOT use the finite phase transition to
  claim release, issue, or implementation completion. An observed `blocked`
  record remains governed by the existing `goal-lifecycle` recovery authority;
  this reference MUST NOT use ordinary completion language to replace, weaken,
  or restate that sequence. During an unfinished active overall goal's external
  wait or Watcher handoff, Codex MUST NOT use administrative completion merely
  to clear the handoff. If the host exposes no cancel, pause, transfer, or
  objective-update operation for that active goal, Codex MUST state the
  limitation and leave the unsupported transition unresolved; it MUST NOT
  promise that an idle Orchestrator will wake later.
- Ordinary app waiting and scheduled follow-up are separate. Codex MUST NOT use
  a heartbeat or automation as an app Watcher workaround. If no supported
  Watcher/wake route exists, Codex MUST record the exact limitation and bounded
  fallback; it MUST NOT invent a monitor identity.
