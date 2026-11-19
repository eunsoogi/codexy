# Codexy hooks

Codex loads this directory as a plugin hook source and substitutes `PLUGIN_ROOT`
before invoking each configured concern launcher. Routing admission hooks are
stateless; supervision capacity persists parent-scoped reservations. A denied
operation emits the official event-native denial schema with its concern's
diagnostic family. The core manifest registers the thread-delivery,
child-thread-creation, supervision-capacity, subagent-ownership, and
cleanup-progress concerns. Watcher waits and session cancellation remain in the
MCP runtime and host transport; the core package does not add `watcher_wait`,
sender `PostToolUse`, or `UserPromptSubmit` cancellation hooks. The cleanup
concern's `Interrupt` handler only suppresses its own continuation.

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

Thread-delivery admission validates a non-empty `threadId`, explicit `model` and
`thinking` fields, and one exact supported pair: `gpt-6.2-sol`/`low`,
`gpt-6.1-terra`/`medium`, or `gpt-6.2-sol`/`medium`. The same request-level
check applies to both hook events and both tool-name aliases. Optional legacy
`codexy_thread_delivery` data does not change the decision. The hook does not
infer a recipient role or verify parent/child identity, target existence, or
recipient assignment. Orchestration selects the known recipient and its assigned
pair; the host authorizes the actual delivery. This hook never opens
`transcript_path` or uses a prompt, message body, or other conversation content
as route authority. Bounded JSON, event, and tool validation remain in force,
including duplicate-key rejection.

The child-thread-creation concern still requires native, non-empty `model` and
`thinking` fields. The plugin admits `spawn_agent` only for exact packaged
Codexy specialists or the explicit read-only `explorer` role. Generic, worker,
omitted, and unknown roles fail closed instead of becoming implementation
owners; durable work MUST use a Codex child thread, and unavailable thread
tooling MUST be reported as a blocker. Allowlisted specialists and `explorer`
remain bounded helpers or reviewers; branch, worktree, pull-request, and
review-response ownership is denied at admission. Native `send_message`,
`followup_task`, and `send_input` requests are denied regardless of content: the
host may deliver opaque messages that cannot be classified reliably. Use a Codex
child chat for work needing follow-up, or start a new bounded helper with the
necessary context. This does not govern app child-chat messaging. Hook
registration alone does not prove that a host dispatches these events. Every new
native `spawn_agent` request must explicitly set `fork_turns="none"`; omitted,
null, history-copying, bounded-positive, and other values fail closed with
guidance to correct the request and provide a self-contained assignment. A
no-history request value alone does not prove what turns the host supplied to
the child. The spawn-message classifier also rejects direct requests to change
source, commit or push changes, or execute tests and checks for any admitted
role. Bounded read-only review and research remain available when the caller
retains editing and execution responsibility. Natural-language cues are
heuristic, so unfamiliar wording may be missed and ambiguous wording may be
denied. An unfinished quote containing task verbs fails closed; a denial
recommends a Codex child chat but never creates or dispatches one. This is a
workflow guardrail, not an unbypassable execution sandbox. Structured
control-plane handoffs require their explicit marker and stable key in the
orchestration validators; ordinary wording never becomes route authority. The
upstream orchestration resolver selects generic, explicit, fallback, and
named-role pairs. The installed Codexy GitHub plugin adds workflow context and
scoped local destructive shell/Git safety through its own `${PLUGIN_ROOT}` hook
manifest. GitHub mutations use the normal host, connector, and GitHub
authorization; the plugin does not require a repository admission file or an
exclusive write route. Repository owners may define their own local hooks
independently. `PreToolUse` and `PermissionRequest` bind the same concern
owners; matching handlers are independent and any denial is conservative.
Malformed governed input fails visibly rather than guessing.

The retained admission launchers run Python isolated from user configuration.
The supervision hook writes only its private plugin-data reservation database;
the routing hooks do not update user state. A trusted operator may set the
absolute `CODEXY_CORE_HOOK_TIMING_FILE` environment variable to enable one
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
