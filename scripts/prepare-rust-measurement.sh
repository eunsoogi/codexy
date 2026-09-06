#!/usr/bin/env bash

set -euo pipefail

mode="${CACHE_MODE:-}"
head_sha="${HEAD_SHA:-}"
condition="${CONDITION:-}"
repeat_id="${REPEAT_ID:-}"
profiling="${PROFILE_ENABLED:-false}"
cache_identity="${CACHE_IDENTITY:-}"
shard_index="${SHARD_INDEX:-}"
runner_temp="${RUNNER_TEMP:-}"
github_env="${GITHUB_ENV:-}"

[[ "$mode" =~ ^(isolated|normal)$ ]]
[[ "$head_sha" =~ ^[0-9a-f]{40}$ ]]
test "$(git rev-parse HEAD)" = "$head_sha"
[[ "$condition" =~ ^(cold|warm)$ ]]
[[ "$repeat_id" =~ ^[A-Za-z0-9._-]{1,64}$ ]]
[[ "$cache_identity" =~ ^[A-Za-z0-9._-]{1,64}$ ]]
[[ "$profiling" =~ ^(true|false)$ ]]
test -n "$runner_temp"
test -n "$github_env"

root="$runner_temp/codexy-rust-measurement"
rm -rf -- "$root"
mkdir -p "$root/metrics"

{
	printf 'head_sha=%s\n' "$head_sha"
	printf 'condition=%s\n' "$condition"
	printf 'cache_mode=%s\n' "$mode"
	printf 'repeat_id=%s\n' "$repeat_id"
	printf 'profiling=%s\n' "$profiling"
	printf 'cache_identity=%s\n' "$cache_identity"
	printf 'shard_index=%s\n' "$shard_index"
	printf 'runner_os=%s\n' "${RUNNER_OS:-unknown}"
} >"$root/metrics/measurement.txt"
[[ "$condition" != cold ]] || printf 'cache_state=cold-empty\n' >>"$root/metrics/measurement.txt"

if [[ "$mode" == isolated ]]; then
	mkdir -p "$root/cargo" "$root/rustup" "$root/target"
	printf 'CARGO_HOME=%s/cargo\nRUSTUP_HOME=%s/rustup\nCARGO_TARGET_DIR=%s/target\n' \
		"$root" "$root" "$root" >>"$github_env"
fi

if [[ "$profiling" == true ]]; then
	printf 'CODEXY_PROFILE_METRICS=%s/metrics/profile.metrics\nCODEXY_PROFILE_COMMAND_METRICS_DIR=%s/metrics/commands\n' \
		"$root" "$root" >>"$github_env"
fi
