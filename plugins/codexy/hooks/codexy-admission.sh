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

# The fixed PATH admits supported macOS Python locations without inheriting user state.
if env -i PATH=/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin python3 -I -B -c \
  'import sys; raise SystemExit(0 if sys.version_info >= (3, 10) else 1)' && \
  env -i PATH=/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin python3 -I -B "${plugin_root}/hooks/codexy-admission.py" \
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
