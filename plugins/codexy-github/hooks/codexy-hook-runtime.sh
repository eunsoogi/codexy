#!/bin/sh
entrypoint=${1-}
event=${2-}
case "$entrypoint" in
  codexy-thread-delivery.py|codexy-repository-issue.py|codexy-repository-pull-request.py|codexy-repository-merge.py|codexy-repository-github-command.py|codexy-destructive-command.py) ;;
  *) exit 1 ;;
esac
case "$event" in
  PreToolUse|PermissionRequest) ;;
  *) exit 1 ;;
esac

plugin_root=${PLUGIN_ROOT-}
[ -n "$plugin_root" ] || plugin_root=${0%/hooks/codexy-hook-runtime.sh}
runtime_home=${HOME-}
runtime_user=${USER-}
set -- /usr/bin/env -i PATH=/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin HOME="$runtime_home" USER="$runtime_user"
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
[ -z "${GH_REPO-}" ] || set -- "$@" "GH_REPO=$GH_REPO"
[ -z "${GIT_DIR-}" ] || set -- "$@" "GIT_DIR=$GIT_DIR"
[ -z "${GIT_COMMON_DIR-}" ] || set -- "$@" "GIT_COMMON_DIR=$GIT_COMMON_DIR"
if [ "${GIT_CONFIG_COUNT+x}" = x ]; then
  set -- "$@" "GIT_CONFIG_COUNT=$GIT_CONFIG_COUNT"
  case "$GIT_CONFIG_COUNT" in
    ''|*[!0-9]*) ;;
    *)
      config_index=0
      while [ "$config_index" -lt "$GIT_CONFIG_COUNT" ] && [ "$config_index" -lt 65 ]; do
        eval "config_key_set=\${GIT_CONFIG_KEY_${config_index}+x}"
        eval "config_value_set=\${GIT_CONFIG_VALUE_${config_index}+x}"
        if [ "$config_key_set" = x ]; then
          eval "config_key=\${GIT_CONFIG_KEY_${config_index}}"
          set -- "$@" "GIT_CONFIG_KEY_${config_index}=$config_key"
        fi
        if [ "$config_value_set" = x ]; then
          eval "config_value=\${GIT_CONFIG_VALUE_${config_index}}"
          set -- "$@" "GIT_CONFIG_VALUE_${config_index}=$config_value"
        fi
        config_index=$((config_index + 1))
      done
      ;;
  esac
fi

output=$("$@" "$python" -I -B "${plugin_root}/hooks/${entrypoint}" --event "$event" 2>/dev/null) || exit 1
[ -z "$output" ] || printf '%s\n' "$output"
