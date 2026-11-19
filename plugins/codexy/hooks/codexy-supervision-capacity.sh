#!/bin/sh
event=${1-}
case "$event" in PreToolUse | PostToolUse) ;; *) event=PreToolUse ;; esac
plugin_root=${PLUGIN_ROOT-}
[ -n "$plugin_root" ] || plugin_root=${0%/hooks/codexy-supervision-capacity.sh}
if "${plugin_root}/hooks/codexy-hook-runtime.sh" codexy_supervision_capacity.py "$event"; then
	exit 0
fi
if [ "$event" = PostToolUse ]; then
	printf '%s\n' '{"systemMessage":"Supervision state is unavailable; retain reservations and reconcile before dispatch."}'
else
	printf '%s\n' '{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"deny","permissionDecisionReason":"CODEXY_SUPERVISION_CAPACITY_RUNTIME: reservation policy unavailable."}}'
fi
