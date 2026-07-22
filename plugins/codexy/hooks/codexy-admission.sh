#!/bin/sh
# Static launcher for the two supported plugin platforms.
set -eu

event=${1-}
case "$event" in
  PreToolUse|PermissionRequest) ;;
  *) event=PreToolUse ;;
esac

plugin_root=${PLUGIN_ROOT-}
if [ -z "$plugin_root" ]; then
  plugin_root=${0%/hooks/codexy-admission.sh}
fi

# The absolute interpreter and plugin-relative source avoid PATH, package managers,
# imports outside this archive, bytecode writes, caches, and network bootstrap.
if env -i /usr/bin/python3 -I -B "${plugin_root}/hooks/codexy-admission.py" \
  --event "$event"; then
  exit 0
fi

case "$event" in
  PreToolUse)
    printf '%s\n' '{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"deny","permissionDecisionReason":"Codexy policy: MUST NOT execute when the static admission runtime is unavailable."}}'
    ;;
  PermissionRequest)
    printf '%s\n' '{"hookSpecificOutput":{"hookEventName":"PermissionRequest","decision":{"behavior":"deny","message":"Codexy policy: MUST NOT execute when the static admission runtime is unavailable."}}}'
    ;;
esac
