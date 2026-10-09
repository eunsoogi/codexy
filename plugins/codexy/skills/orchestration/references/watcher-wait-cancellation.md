# Watcher wait cancellation

The core `PreToolUse` hook binds an authenticated `watcher_wait` call to an
opaque, short-lived request record. A same-connection MCP cancellation or a
packaged Watcher cancellation hook releases only that request; it preserves the
durable Watcher session and its event log.

When a JSON wait result is returned, `cancellationReason` appears only with
`status=cancelled`: `request_cancelled` identifies the request-binding branch,
and `session_cancelled` identifies durable `watcher_cancel`. The runtime derives
this value from the branch that returned the result. A same-connection MCP
`notifications/cancelled` response may be suppressed by the transport, so it
does not produce a synthetic JSON result or cancellation reason.

The `Interrupt` hook matches the exact host `session_id` and `turn_id`. The
`UserPromptSubmit` hook matches the host session and cancels only when exactly
one armed or active Watcher wait is bound to that session. If more than one
request matches, it fails closed. The hook forwards only `session_id`; prompt
text does not reach the Watcher runtime. Direct Watcher callers remain
binding-free.

The host MUST deliver a matching hook for plugin cancellation to take effect. A
delegated task message or outer wait termination may leave the native request
active. Codex MUST keep host observations tied to the exact candidate and
distinguish these results:

- An earlier candidate exposed `UserPromptSubmit` as untrusted. A delegated
  task-to-task follow-up timed out, and its read-thread record omitted
  `requestBinding`. That observation does not establish trusted `PreToolUse`
  injection or host prompt delivery, and it is not a cancellation pass.
- On the parent task's current Codexy `1.13.1` production session, a human
  prompt arrived while the native `watcher_wait` used cursor `5`; the recorded
  call included an injected `requestBinding`. Its tool record completed after
  `141581` ms, and the parent task readback reported `status=cancelled`,
  `events=[]`, and `nextCursor=5`. The same-session health readback reported
  `active`, generation 1, `waiting=false`, cursor 5. The parent later reused the
  session with waits starting at cursors 5, 6, and 7 and reports that the
  existing Watcher delivered events 6 and 7. This supports same-task human-input
  cancellation and subsequent cursor/session continuity. The task transcript
  does not expose the native result payloads, exact input and return times, or
  the low-level cancellation event. Codex MUST NOT infer `UserPromptSubmit`,
  `Interrupt`, or MCP cancellation from `status=cancelled`.
- On Codexy `1.13.1` at source checkout `95b47dde`, with Codex CLI `0.160.0`, a
  coordinator message sent through the Codex app reached a separate diagnostic
  task while its native `watcher_wait` was pending. The task readback reported
  an injected `requestBinding`, but the wait returned `status=timeout`,
  `events=[]`, and `nextCursor=0` after `295037` ms. The primary session and
  independent control session both read back `active` at generation 1 with
  `waiting=false`; the primary cursor remained 0. The diagnostic task's final
  readback reports call start `2026-10-05
  03:41:33.722 UTC` and return
  `03:46:28.858 UTC`; its MCP tool record reports `295037` ms. Those reported
  timestamps and duration differ by about 99 ms, and the parent read-thread
  record does not expose raw tool timestamps, so retain each value with its
  source rather than deriving one from the other. The `03:41:10 UTC` arm
  announcement preceded the call; the `03:41:56 UTC` message-send marker does
  not expose delivery time, so message-to-return latency is unavailable. This
  confirms only the cross-task message route on this candidate. Codex MUST keep
  it distinct from the same-task human prompt result above; it does not test
  physical UI input or identify a cancellation hook.
- On trusted candidate `32d205c`, a real native wait acquired no
  `requestBinding`. The macOS launcher selected Python running as x86_64 under
  Rosetta; the hook exited successfully without finding the installed
  darwin-arm64 runtime. A subsequent app-delivered input left the wait pending
  after five seconds. This is a reproduced host failure.
- Codex MUST keep same-task human prompts, cross-task messages, physical desktop
  input, `Interrupt`, and MCP cancellation as distinct evidence. The
  observations above do not identify which low-level event caused the same-task
  result or whether physical desktop input follows it. Codex MUST record event
  provenance and input-to-return timing when the host exposes them, and MUST NOT
  infer either from an outer interruption or tool yield. A future probe MUST
  have a changed candidate or a specific unresolved acceptance criterion; a
  launcher regression test alone does not establish host delivery.

`watcher_cancel` is a different operation: it durably ends the session and
requires a new assignment. A cancelled queue or cursor is not continuity, and
Codex MUST NOT resume the old assignment.
