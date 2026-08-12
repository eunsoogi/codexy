#!/bin/sh
# Native Codex plugin hook. Codex resolves PLUGIN_ROOT before launching this file.
set -efu
deny() {
  printf '%s\n' '{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"deny","permissionDecisionReason":"CODEXY_GITHUB_ADMISSION: package runtime is unavailable"}}'
}
[ -n "${PLUGIN_ROOT-}" ] || { deny; exit 0; }
case "${1-}:${2-}" in --rule:issue|--rule:pr) ;; *) exit 0 ;; esac
[ -x /usr/bin/python3 ] || { deny; exit 0; }
exec /usr/bin/python3 -I -B "$PLUGIN_ROOT/hooks/codexy-github-admission.py" --rule "$2"
