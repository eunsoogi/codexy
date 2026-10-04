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
active. Keep host observations tied to the exact candidate and distinguish these
results:

- An earlier candidate exposed `UserPromptSubmit` as untrusted. A delegated
  task-to-task follow-up timed out, and its read-thread record omitted
  `requestBinding`. That observation does not establish trusted `PreToolUse`
  injection or host prompt delivery, and it is not a cancellation pass.
- On trusted candidate `32d205c`, a real native wait acquired no
  `requestBinding`. The macOS launcher selected Python running as x86_64 under
  Rosetta; the hook exited successfully without finding the installed
  darwin-arm64 runtime. A subsequent app-delivered input left the wait pending
  after five seconds. This is a reproduced host failure.
- After a repaired exact candidate is installed, repeat the test by submitting
  input through a supported host/app path and verify prompt release, durable
  session continuity, and isolation. The parent may drive this path
  automatically; the input need not be physically typed. Bind the result to the
  exact source and runtime identities. A launcher regression test alone does not
  establish host delivery.

`watcher_cancel` is a different operation: it durably ends the session and
requires a new assignment. A cancelled queue or cursor is not continuity, and
the old assignment must not resume.
