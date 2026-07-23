#!/bin/sh
# Static launcher for the two supported plugin platforms.
set -eu

event=${1-}
case "$event" in
  PreToolUse|PermissionRequest) ;;
  *) event=PreToolUse ;;
esac

plugin_root=${PLUGIN_ROOT-}
if [ -z "$plugin_root" ]; then
  plugin_root=${0%/hooks/codexy-admission.sh}
fi

# The fixed PATH admits supported macOS tools; only selectors needed for
# effective policy cross the isolated launcher boundary.
runtime_home=${HOME-}
set -- env -i PATH=/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin HOME="$runtime_home"
[ -z "${GH_REPO-}" ] || set -- "$@" "GH_REPO=$GH_REPO"
[ -z "${GIT_DIR-}" ] || set -- "$@" "GIT_DIR=$GIT_DIR"
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
if env -i PATH=/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin HOME="$runtime_home" python3 -I -B -c \
  'import sys; raise SystemExit(0 if sys.version_info >= (3, 10) else 1)' && \
  "$@" python3 -I -B "${plugin_root}/hooks/codexy-admission.py" \
  --event "$event"; then
  exit 0
fi

case "$event" in
  PreToolUse)
    printf '%s\n' '{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"deny","permissionDecisionReason":"Codexy policy: MUST NOT execute when the static admission runtime is unavailable."}}'
    ;;
  PermissionRequest)
    printf '%s\n' '{"hookSpecificOutput":{"hookEventName":"PermissionRequest","decision":{"behavior":"deny","message":"Codexy policy: MUST NOT execute when the static admission runtime is unavailable."}}}'
    ;;
esac
