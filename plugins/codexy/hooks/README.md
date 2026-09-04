# Codexy hooks

Codex loads this directory as a plugin hook source and substitutes `PLUGIN_ROOT`
before invoking each configured concern launcher. The hooks are stateless: a
permitted operation writes zero bytes; a denied operation emits only the
official event-native denial schema with its concern's diagnostic family.

The installed Codexy plugin activates thread-delivery metadata and
child-thread-creation admission requiring native, non-empty `model` and
`thinking` fields. It also admits `spawn_agent` only for exact packaged Codexy
specialists or the explicit read-only `explorer` role. Generic, worker, omitted,
and unknown roles fail closed instead of becoming implementation owners; durable
work MUST use a Codex child thread, and unavailable thread tooling MUST be
reported as a blocker. The upstream orchestration resolver selects generic,
explicit, fallback, and named-role pairs; the native host owns the callable
pair, and this hook does not infer or authenticate route provenance. The
installed Codexy GitHub plugin activates generic GitHub-command and destructive
shell/Git admission through its own `${PLUGIN_ROOT}` hook manifest. A trusted
repository's `.codex/hooks.json` activates only its repository-specific issue,
pull-request, merge, and release governance. `PreToolUse` and
`PermissionRequest` bind the same concern owners; matching handlers are
independent and any denial is conservative. Malformed governed input fails
visibly rather than guessing.

The launchers run Python isolated from user configuration and never install,
cache, update, or mutate user state. Their configured outer hook timeout bounds
execution; if the static runtime is unavailable, they fail closed with the
matching event-native denial. Plugin hooks require Codex trust for their exact
hash and are excluded when an administrator enables managed-hooks-only mode.

These checks do not claim to enforce labels, reviews, CI, owner/Sentinel state,
or prior tool use because those facts are not authoritative hook input.
