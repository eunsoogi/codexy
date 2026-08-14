#!/bin/sh
entrypoint=${1-}
event=${2-}
[ "$entrypoint" = codexy-thread-delivery.py ] || exit 1
case "$event" in PreToolUse|PermissionRequest) ;; *) exit 1 ;; esac

plugin_root=${PLUGIN_ROOT-}
[ -n "$plugin_root" ] || plugin_root=${0%/hooks/codexy-hook-runtime.sh}
runtime_home=${HOME-}
runtime_user=${USER-}
python=
for candidate in /usr/local/bin/python3 /usr/bin/python3; do
  [ -x "$candidate" ] || continue
  if /usr/bin/env -i PATH=/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin HOME="$runtime_home" USER="$runtime_user" "$candidate" -I -B -c \
    'import sys; raise SystemExit(0 if sys.version_info >= (3, 10) else 1)' 2>/dev/null
  then
    python=$candidate
    break
  fi
done
[ -n "$python" ] || exit 1
output=$(/usr/bin/env -i PATH=/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin HOME="$runtime_home" USER="$runtime_user" \
  "$python" -I -B "${plugin_root}/hooks/${entrypoint}" --event "$event" 2>/dev/null) || exit 1
[ -z "$output" ] || printf '%s\n' "$output"
