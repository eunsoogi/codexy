#!/bin/sh
event=${1-}
case "$event" in
PostToolUse | Stop | Interrupt) ;;
*) exit 0 ;;
esac
plugin_root=${PLUGIN_ROOT-}
[ -n "$plugin_root" ] || plugin_root=${0%/hooks/codexy-cleanup-progress.sh}
if "${plugin_root}/hooks/codexy-hook-runtime.sh" codexy_cleanup_progress.py "$event"; then
	exit 0
fi
# Failure never invents an actionable cleanup step.
printf '%s\n' '{}'
