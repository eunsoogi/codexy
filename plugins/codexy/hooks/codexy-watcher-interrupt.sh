#!/bin/sh
event=${1-}
# Limit this best-effort adapter to watcher wait lifecycle events.
case "$event" in
PreToolUse | Interrupt | UserPromptSubmit) ;;
*) exit 0 ;;
esac
plugin_root=${PLUGIN_ROOT-}
[ -n "$plugin_root" ] || plugin_root=${0%/hooks/codexy-watcher-interrupt.sh}
# Cancellation notifications must not make the calling Codex hook fail closed.
"${plugin_root}/hooks/codexy-hook-runtime.sh" codexy_watcher_interrupt.py "$event" || true
exit 0
