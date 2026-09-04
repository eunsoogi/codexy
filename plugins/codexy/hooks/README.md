# Codexy hooks

Codex loads this directory as a plugin hook source and substitutes `PLUGIN_ROOT`
before invoking each configured concern launcher. The hooks are stateless: a
permitted operation writes zero bytes; a denied operation emits only the
official event-native denial schema with its concern's diagnostic family.

The installed Codexy plugin activates thread-delivery admission through a
bounded, host-authenticated `codexy_thread_delivery` envelope. The envelope
binds the current `session_id`, the requested `threadId`, the direction, and
the target's explicit model/thinking pair. Root-to-child delivery requires
`gpt-5.6-luna`/`max`; child-to-parent delivery requires
`gpt-5.6-sol`/`medium`. Missing, malformed, ambiguous, or mismatched metadata
fails closed. This hook never opens `transcript_path` and never uses a prompt,
message body, or other conversation content as route authority. The native
host owns production of the authenticated envelope.

The child-thread-creation concern still requires native, non-empty `model` and
`thinking` fields. The installed Codexy GitHub plugin activates generic
GitHub-command and destructive shell/Git admission through its own
`${PLUGIN_ROOT}` hook manifest. A trusted repository's `.codex/hooks.json`
activates only its repository-specific issue, pull-request, merge, and release
governance. `PreToolUse` and `PermissionRequest` bind the same concern owners;
matching handlers are independent and any denial is conservative. Malformed
governed input fails visibly rather than guessing.

The launchers run Python isolated from user configuration and never install,
cache, update, or mutate user state. Their configured outer hook timeout bounds
execution; if the static runtime is unavailable, they fail closed with the
matching event-native denial. Plugin hooks require Codex trust for their exact
hash and are excluded when an administrator enables managed-hooks-only mode.

These checks do not claim to enforce labels, reviews, CI, owner/Sentinel state,
or prior tool use because those facts are not authoritative hook input.
