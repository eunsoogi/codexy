#!/bin/sh
set -efu

event=${1-}
kind=${2-}
case "$event" in
PreToolUse | PermissionRequest) ;;
*) event=PreToolUse ;;
esac
case "$kind" in
issue | pr | merge | shell | nested) ;;
*) kind=shell ;;
esac
plugin_root=${PLUGIN_ROOT-}
[ -n "$plugin_root" ] || plugin_root=${0%/hooks/codexy-title-check.sh}
if "${plugin_root}/hooks/codexy-hook-runtime.sh" codexy-title-check.py "$event" "$kind"; then
	exit 0
fi
if [ "$event" = PermissionRequest ]; then
	printf '%s\n' '{"hookSpecificOutput":{"hookEventName":"PermissionRequest","decision":{"behavior":"deny","message":"CODEXY_TITLE_CHECK_RUNTIME: Codexy title validation could not run."}}}'
else
	printf '%s\n' '{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"deny","permissionDecisionReason":"CODEXY_TITLE_CHECK_RUNTIME: Codexy title validation could not run."}}'
fi
