# Watcher wait cancellation

The core `PreToolUse` hook binds an authenticated `watcher_wait` call to an
opaque, short-lived request record. A same-connection MCP cancellation or a
packaged Watcher cancellation hook releases only that request; it preserves the
durable Watcher session and its event log.

The `Interrupt` hook matches the exact host `session_id` and `turn_id`. The
`UserPromptSubmit` hook matches the host session and cancels only when exactly
one armed or active Watcher wait is bound to that session. If more than one
request matches, it fails closed. The hook forwards only `session_id`; prompt
text does not reach the Watcher runtime. Direct Watcher callers remain
binding-free.

The host must deliver a matching hook for plugin cancellation to take effect. A
delegated task message or outer wait termination may leave the native request
active. The recorded candidate probe used a real native Watcher wait and a
delivered task-to-task follow-up; that wait timed out instead of cancelling. The
read-thread tool record omitted `requestBinding`, which does not establish
whether the host ran or applied `PreToolUse` injection. This probe therefore
does not verify a genuine human `UserPromptSubmit` path. Parent-owned candidate
installation and actual human-input propagation remain separate acceptance
evidence.

`watcher_cancel` is a different operation: it durably ends the session and
requires a new assignment. A cancelled queue or cursor is not continuity, and
the old assignment must not resume.
