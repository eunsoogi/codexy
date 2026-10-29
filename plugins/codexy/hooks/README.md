# Codexy hooks

Codex loads this directory as a plugin hook source and substitutes `PLUGIN_ROOT`
before invoking each configured concern launcher. Admission hooks are stateless:
a permitted operation writes zero bytes; a denied operation emits only the
official event-native denial schema with its concern's diagnostic family. The
core manifest registers the thread-delivery, child-thread-creation,
subagent-ownership, and cleanup-progress concerns. Watcher waits and session
cancellation remain in the MCP runtime and host transport; the core package does
not add `watcher_wait`, sender `PostToolUse`, or `UserPromptSubmit` cancellation
hooks. The cleanup concern's `Interrupt` handler only suppresses its own
continuation.

Cleanup progress observes an exact thread archive request returning a non-error
MCP envelope. This does not establish archival, identity, acceptance, or
eligibility. On the same turn, Stop can request one `list_archived_threads`
readback; an observed list attempt or interruption suppresses that request. It
never performs cleanup or infers success from an undocumented result body.
Unknown input or unavailable private state remains non-blocking. Turn markers
are stored in the host-provided plugin data directory and expire after one day.
POSIX uses ownership and mode checks; Windows uses the host-provided data root.
The parent remains responsible for interpreting readbacks and performing
cleanup.

When the native host supplies a bounded, host-authenticated
`codexy_thread_delivery` v2 envelope, the installed Codexy plugin validates it
for thread-delivery admission. The envelope binds the current `session_id`, the
requested `threadId`, the direction, and the target's explicit model/thinking
pair. Its authenticated sender/target pair carries the native parent/child
correlation without replaying a transcript. Root-to-child delivery requires
`gpt-6-luna`/`max`; child-to-parent delivery also requires `gpt-6-luna`/`max`. A
legacy host that omits the top-level envelope retains the prior no-op admission
path until host rollout; present but malformed, ambiguous, or mismatched v2
metadata fails closed. This hook never opens `transcript_path` and never uses a
prompt, message body, or other conversation content as route authority. The
native host owns production of the authenticated envelope.

The child-thread-creation concern still requires native, non-empty `model` and
`thinking` fields. The plugin admits `spawn_agent` only for exact packaged
Codexy specialists or the explicit read-only `explorer` role. Generic, worker,
omitted, and unknown roles fail closed instead of becoming implementation
owners; durable work MUST use a Codex child thread, and unavailable thread
tooling MUST be reported as a blocker. Allowlisted specialists and `explorer`
remain bounded helpers or reviewers; branch, worktree, pull-request, and
review-response ownership is denied at admission. Both `send_message` and
`followup_task` apply the same message classifier to every nonempty target. The
follow-up envelope does not authenticate target existence or role, so the hook
never relies on a role claimed in the message; malformed targets fail closed.
Structured control-plane handoffs require their explicit marker and stable key
in the orchestration validators; ordinary wording never becomes route authority.
The upstream orchestration resolver selects generic, explicit, fallback, and
named-role pairs. The installed Codexy GitHub plugin adds workflow context and
scoped local destructive shell/Git safety through its own `${PLUGIN_ROOT}` hook
manifest. GitHub mutations use the normal host, connector, and GitHub
authorization; the plugin does not require a repository admission file or an
exclusive write route. Repository owners may define their own local hooks
independently. `PreToolUse` and `PermissionRequest` bind the same concern
owners; matching handlers are independent and any denial is conservative.
Malformed governed input fails visibly rather than guessing.

The retained admission launchers run Python isolated from user configuration and
never install, cache, update, or mutate user state. A trusted operator may set
the absolute `CODEXY_CORE_HOOK_TIMING_FILE` environment variable to enable one
bounded local JSONL record per core-hook decision. Each record contains only
`event`, `concern`, integer nanosecond `elapsed`, and `decision`; the payload
cannot choose the setting or path. The file is created private to its owner,
rejects symlinks, non-regular or permissive targets, and stops before exceeding
1 MiB. Timing write failures do not change admission, and the setting is not
forwarded from the hook payload. The timing file's parent must already exist.

The configured outer hook timeout bounds execution; if the static runtime is
unavailable, the launchers fail closed with the matching event-native denial.
Plugin hooks require Codex trust for their exact hash and are excluded when an
administrator enables managed-hooks-only mode.

These checks do not claim to enforce labels, reviews, CI, owner/Sentinel state,
or prior tool use because those facts are not authoritative hook input.
