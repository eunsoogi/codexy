#!/usr/bin/env bash

set -euo pipefail

mode="${CODEXY_MEASUREMENT_MODE:-}"
condition="${CODEXY_MEASUREMENT_CONDITION:-}"
cache_hit="${CODEXY_MEASUREMENT_CACHE_HIT:-}"

add_isolated_state() {
	local state="$1"
	local root="${CODEXY_MEASUREMENT_ROOT:-}"
	local measurement_file="$root/metrics/measurement.txt"
	test -n "$root"
	test -f "$measurement_file"
	printf 'cache_state=%s\n' "$state" >>"$measurement_file"
}

case "$mode:$condition" in
normal:cold)
	case "$cache_hit" in
	"" | false)
		printf 'normal cold measurement confirmed cache miss (cache-hit=%s)\n' "${cache_hit:-empty}"
		;;
	true)
		printf 'normal cold measurement requires an exact cache miss (cache-hit=true)\n' >&2
		exit 1
		;;
	*)
		printf 'normal cold measurement received an unexpected cache-hit value: %s\n' "$cache_hit" >&2
		exit 1
		;;
	esac
	;;
normal:warm)
	if [[ "$cache_hit" != true ]]; then
		printf 'normal warm measurement requires an exact cache hit (cache-hit=%s)\n' "${cache_hit:-empty}" >&2
		exit 1
	fi
	printf 'normal warm measurement confirmed exact cache hit\n'
	;;
isolated:cold)
	;;
isolated:warm)
	if [[ "$cache_hit" == true ]]; then
		add_isolated_state warm-hit
	else
		add_isolated_state warm-miss
		printf 'isolated warm measurement requires an exact cache hit\n' >&2
		exit 1
	fi
	;;
*)
	printf 'unsupported measurement cache mode or condition: %s:%s\n' "$mode" "$condition" >&2
	exit 1
	;;
esac
