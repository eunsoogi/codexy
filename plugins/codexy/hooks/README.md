# Codexy hooks

Codex loads this directory as a plugin hook source and substitutes `PLUGIN_ROOT`
before invoking the configured static launcher. The dispatchers are stateless:
a permitted operation writes zero bytes; a denied operation emits only the
official event-native denial schema.

`PreToolUse` checks only complete current input for the owned repository: typed
GitHub title and squash-merge data, explicit recipient `model` and `thinking`,
and structurally parsed destructive shell/Git operations. `PermissionRequest`
uses the same deny-only policy. Other repositories and unrelated tools pass
without context. Malformed owned input fails visibly rather than guessing.

The launchers run Python isolated from user configuration and never install,
cache, update, or mutate user state. Their configured outer hook timeout bounds
execution; if the static runtime is unavailable, they fail closed with the
matching event-native denial. Plugin hooks require Codex trust for their exact
hash and are excluded when an administrator enables managed-hooks-only mode.

Codex 0.144.4 supports manual and automatic PreCompact/PostCompact triggers,
but neither output schema can add model-visible developer context. No compact
handler is configured, and issue #455 tracks that upstream capability instead
of emulating success with SessionStart or UserPromptSubmit.

These checks do not claim to enforce labels, reviews, CI, owner/Sentinel state,
or prior tool use because those facts are not authoritative hook input.
