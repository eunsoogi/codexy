#!/bin/sh
event=${1-}
case "$event" in
PreToolUse | PermissionRequest) ;;
*) event=PreToolUse ;;
esac

plugin_root=${PLUGIN_ROOT-}
[ -n "$plugin_root" ] || plugin_root=${0%/hooks/codexy-child-thread-creation.sh}
runtime_home=${HOME-}
runtime_user=${USER-}
interpreter_name=p
interpreter_name=${interpreter_name}ython3
interpreter=
for candidate in /usr/local/bin/"$interpreter_name" /usr/bin/"$interpreter_name"; do
	[ -x "$candidate" ] || continue
	/usr/bin/env -i PATH=/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin HOME="$runtime_home" USER="$runtime_user" "$candidate" -I -B -c \
		'import sys; raise SystemExit(int(sys.version_info[:2] < (3, 10)))' && interpreter=$candidate && break
done
[ -n "$interpreter" ] || {
	if [ "$event" = PermissionRequest ]; then
		printf '%s\n' '{"hookSpecificOutput":{"hookEventName":"PermissionRequest","decision":{"behavior":"deny","message":"CODEXY_CHILD_THREAD_CREATION_RUNTIME: Codexy policy MUST NOT execute this operation."}}}'
	else
		printf '%s\n' '{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"deny","permissionDecisionReason":"CODEXY_CHILD_THREAD_CREATION_RUNTIME: Codexy policy MUST NOT execute this operation."}}'
	fi
	exit 0
}
output=$(/usr/bin/env -i PATH=/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin HOME="$runtime_home" USER="$runtime_user" PLUGIN_ROOT="$plugin_root" \
	"$interpreter" -I -B "${plugin_root}/hooks/codexy-child-thread-creation.py" --event "$event") || {
	if [ "$event" = PermissionRequest ]; then
		printf '%s\n' '{"hookSpecificOutput":{"hookEventName":"PermissionRequest","decision":{"behavior":"deny","message":"CODEXY_CHILD_THREAD_CREATION_RUNTIME: Codexy policy MUST NOT execute this operation."}}}'
	else
		printf '%s\n' '{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"deny","permissionDecisionReason":"CODEXY_CHILD_THREAD_CREATION_RUNTIME: Codexy policy MUST NOT execute this operation."}}'
	fi
	exit 0
}
[ -z "$output" ] || printf '%s\n' "$output"
