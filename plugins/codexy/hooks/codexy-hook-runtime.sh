#!/bin/sh
entrypoint=${1-}
event=${2-}
case "$entrypoint" in
codexy-child-thread-creation.py | codexy-subagent-ownership.py | codexy-thread-delivery.py | codexy_watcher_interrupt.py) ;;
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
xdg_state_home=${XDG_STATE_HOME-}
xdg_cache_home=${XDG_CACHE_HOME-}
runtime_dir=${CODEXY_RUNTIME_DIR-}
runtime_cache_dir=${CODEXY_RUNTIME_CACHE_DIR-}
runtime_platform=${CODEXY_RUNTIME_PLATFORM-}
runtime_package_sha256=${CODEXY_RUNTIME_PACKAGE_SHA256-}
runtime_git_repository=${CODEXY_RUNTIME_GIT_REPOSITORY-}
runtime_git_ref=${CODEXY_RUNTIME_GIT_REF-}
runtime_source_override=
if [ "${CODEXY_RUNTIME_PACKAGE_PATH+x}" = x ] ||
	[ "${CODEXY_RUNTIME_PACKAGE_URL+x}" = x ] ||
	[ "${CODEXY_RUNTIME_ARTIFACTS_API_URL+x}" = x ]; then
	runtime_source_override=1
fi
set -- \
	"PATH=/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin" \
	"HOME=$runtime_home" \
	"USER=$runtime_user" \
	"PLUGIN_ROOT=$plugin_root"
[ -n "$timing_file" ] && set -- "$@" "CODEXY_CORE_HOOK_TIMING_FILE=$timing_file"
[ -n "$watcher_state_dir" ] && set -- "$@" "CODEXY_WATCHER_STATE_DIR=$watcher_state_dir"
[ -n "$xdg_state_home" ] && set -- "$@" "XDG_STATE_HOME=$xdg_state_home"
[ -n "$xdg_cache_home" ] && set -- "$@" "XDG_CACHE_HOME=$xdg_cache_home"
[ -n "$runtime_dir" ] && set -- "$@" "CODEXY_RUNTIME_DIR=$runtime_dir"
[ -n "$runtime_cache_dir" ] && set -- "$@" "CODEXY_RUNTIME_CACHE_DIR=$runtime_cache_dir"
[ -n "$runtime_platform" ] && set -- "$@" "CODEXY_RUNTIME_PLATFORM=$runtime_platform"
[ -n "$runtime_package_sha256" ] && set -- "$@" "CODEXY_RUNTIME_PACKAGE_SHA256=$runtime_package_sha256"
[ -n "$runtime_git_repository" ] && set -- "$@" "CODEXY_RUNTIME_GIT_REPOSITORY=$runtime_git_repository"
[ -n "$runtime_git_ref" ] && set -- "$@" "CODEXY_RUNTIME_GIT_REF=$runtime_git_ref"
[ -n "$runtime_source_override" ] && set -- "$@" "CODEXY_RUNTIME_SOURCE_OVERRIDE=1"
for candidate in /usr/local/bin/python3 /usr/bin/python3; do
	[ -x "$candidate" ] || continue
	/usr/bin/env -i "$@" \
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
