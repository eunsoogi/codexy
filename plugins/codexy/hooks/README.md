# Codexy hooks

Codex loads this directory as a plugin hook source and substitutes `PLUGIN_ROOT`
before invoking each configured concern launcher. The hooks are stateless: a
permitted operation writes zero bytes; a denied operation emits only the
official event-native denial schema with its concern's diagnostic family.

When the native host supplies a bounded, host-authenticated
`codexy_thread_delivery` v2 envelope, the installed Codexy plugin validates it
for thread-delivery admission. The envelope binds the current `session_id`, the
requested `threadId`, the direction, and the target's explicit model/thinking
pair. Its authenticated sender/target pair carries the native parent/child
correlation without replaying a transcript. Root-to-child delivery requires
`gpt-5.6-luna`/`max`; child-to-parent delivery requires `gpt-6-astra`/`medium`.
A legacy host that omits the top-level envelope retains the prior no-op
admission path until host rollout; present but malformed, ambiguous, or
mismatched v2 metadata fails closed. This hook never opens `transcript_path` and
never uses a prompt, message body, or other conversation content as route
authority. The native host owns production of the authenticated envelope.

The child-thread-creation concern still requires native, non-empty `model` and
`thinking` fields. The plugin admits `spawn_agent` only for exact packaged
Codexy specialists or the explicit read-only `explorer` role. Generic, worker,
omitted, and unknown roles fail closed instead of becoming implementation
owners; durable work MUST use a Codex child thread, and unavailable thread
tooling MUST be reported as a blocker. Allowlisted specialists and `explorer`
remain bounded helpers or reviewers; branch, worktree, pull-request, and
review-response ownership is denied at admission. Structured control-plane
handoffs require their explicit marker and stable key in the orchestration
validators; ordinary wording never becomes route authority. The upstream
orchestration resolver selects generic, explicit, fallback, and named-role
pairs. The installed Codexy GitHub plugin activates generic GitHub-command and
destructive shell/Git admission through its own `${PLUGIN_ROOT}` hook manifest.
A trusted repository's `.codex/hooks.json` activates only its
repository-specific issue, pull-request, merge, and release governance.
`PreToolUse` and `PermissionRequest` bind the same concern owners; matching
handlers are independent and any denial is conservative. Malformed governed
input fails visibly rather than guessing.

The launchers run Python isolated from user configuration and never install,
cache, update, or mutate user state. Their configured outer hook timeout bounds
execution; if the static runtime is unavailable, they fail closed with the
matching event-native denial. Plugin hooks require Codex trust for their exact
hash and are excluded when an administrator enables managed-hooks-only mode.

These checks do not claim to enforce labels, reviews, CI, owner/Sentinel state,
or prior tool use because those facts are not authoritative hook input.
