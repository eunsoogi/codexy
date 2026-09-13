#!/bin/sh
event=${1-}
case "$event" in
PreToolUse | Interrupt) ;;
*) exit 0 ;;
esac
plugin_root=${PLUGIN_ROOT-}
[ -n "$plugin_root" ] || plugin_root=${0%/hooks/codexy-watcher-interrupt.sh}
"${plugin_root}/hooks/codexy-hook-runtime.sh" codexy_watcher_interrupt.py "$event" || true
exit 0
