#!/bin/sh
entrypoint=${1-}
event=${2-}
case "$entrypoint" in
codexy-child-thread-creation.py | codexy-subagent-ownership.py | codexy-thread-delivery.py | codexy-watcher-interrupt.py) ;;
*) exit 1 ;;
esac
case "$event" in
PreToolUse | PermissionRequest | Interrupt) ;;
*) exit 1 ;;
esac

plugin_root=${PLUGIN_ROOT-}
[ -n "$plugin_root" ] || plugin_root=${0%/hooks/codexy-hook-runtime.sh}
runtime_home=${HOME-}
runtime_user=${USER-}
timing_file=${CODEXY_CORE_HOOK_TIMING_FILE-}
watcher_state_dir=${CODEXY_WATCHER_STATE_DIR-}
runtime_dir=${CODEXY_RUNTIME_DIR-}
for candidate in /usr/local/bin/python3 /usr/bin/python3; do
	[ -x "$candidate" ] || continue
	/usr/bin/env -i PATH=/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin HOME="$runtime_home" USER="$runtime_user" PLUGIN_ROOT="$plugin_root" CODEXY_CORE_HOOK_TIMING_FILE="$timing_file" CODEXY_WATCHER_STATE_DIR="$watcher_state_dir" CODEXY_RUNTIME_DIR="$runtime_dir" \
		"$candidate" -I -B "${plugin_root}/hooks/${entrypoint}" --event "$event" \
		2>/dev/null
	status=$?
	case "$status" in
	0) exit 0 ;;
	125) ;;
	*) exit "$status" ;;
	esac
done
exit 1
