#!/bin/sh
event=${1-}
case "$event" in
PreToolUse | PermissionRequest) ;;
*) event=PreToolUse ;;
esac
plugin_root=${PLUGIN_ROOT-}
[ -n "$plugin_root" ] || plugin_root=${0%/hooks/codexy-destructive-command.sh}
if "${plugin_root}/hooks/codexy-hook-runtime.sh" codexy-destructive-command.py "$event"; then
	exit 0
fi
if [ "$event" = PermissionRequest ]; then
	printf '%s\n' '{"hookSpecificOutput":{"hookEventName":"PermissionRequest","decision":{"behavior":"deny","message":"CODEXY_DESTRUCTIVE_COMMAND_RUNTIME: Codexy policy MUST NOT execute this operation."}}}'
else
	printf '%s\n' '{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"deny","permissionDecisionReason":"CODEXY_DESTRUCTIVE_COMMAND_RUNTIME: Codexy policy MUST NOT execute this operation."}}'
fi
