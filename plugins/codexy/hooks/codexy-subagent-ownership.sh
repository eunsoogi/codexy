#!/bin/sh
event=${1-}
case "$event" in
PreToolUse | PermissionRequest) ;;
*) event=PreToolUse ;;
esac

plugin_root=${PLUGIN_ROOT-}
[ -n "$plugin_root" ] || plugin_root=${0%/hooks/codexy-subagent-ownership.sh}
if "${plugin_root}/hooks/codexy-hook-runtime.sh" codexy-subagent-ownership.py "$event"; then
	exit 0
fi
if [ "$event" = PermissionRequest ]; then
	printf '%s\n' '{"hookSpecificOutput":{"hookEventName":"PermissionRequest","decision":{"behavior":"deny","message":"CODEXY_SUBAGENT_OWNERSHIP_RUNTIME: Codexy policy MUST NOT execute this operation."}}}'
else
	printf '%s\n' '{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"deny","permissionDecisionReason":"CODEXY_SUBAGENT_OWNERSHIP_RUNTIME: Codexy policy MUST NOT execute this operation."}}'
fi
