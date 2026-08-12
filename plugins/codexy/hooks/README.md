# Codexy hooks

Codex loads this directory as a plugin hook source and substitutes `PLUGIN_ROOT`
before invoking each configured concern launcher. The hooks are stateless: a
permitted operation writes zero bytes; a denied operation emits only the
official event-native denial schema with its concern's diagnostic family.

The installed plugin activates thread-delivery metadata. Repository issue,
pull-request, merge, GitHub-command, and destructive shell/Git concerns remain
generic implementations for a trusted repository's `.codex/hooks.json` to
activate. `PreToolUse` and `PermissionRequest` bind the same concern owners;
matching handlers are independent and any denial is conservative. Malformed
governed input fails visibly rather than guessing.

The launchers run Python isolated from user configuration and never install,
cache, update, or mutate user state. Their configured outer hook timeout bounds
execution; if the static runtime is unavailable, they fail closed with the
matching event-native denial. Plugin hooks require Codex trust for their exact
hash and are excluded when an administrator enables managed-hooks-only mode.

These checks do not claim to enforce labels, reviews, CI, owner/Sentinel state,
or prior tool use because those facts are not authoritative hook input.
