#!/usr/bin/env bash

set -euo pipefail

condition="${CODEXY_MEASUREMENT_CONDITION:-}"
if [[ "$condition" != cold && "$condition" != warm ]]; then
	printf 'unsupported measurement condition: %s\n' "$condition" >&2
	exit 1
fi

config="packages/codexy-runtime/rust-toolchain.toml"
test -f "$config"

toolchain="$(sed -n 's/^channel = "\([^"]*\)"/\1/p' "$config")"
profile="$(sed -n 's/^profile = "\([^"]*\)"/\1/p' "$config")"
test -n "$toolchain"
test -n "$profile"

component_args=()
while IFS= read -r component; do
	component_args+=(--component "$component")
done < <(sed -n 's/^components = \[\(.*\)\]$/\1/p' "$config" | tr ',' '\n' | sed 's/[[:space:]\"]//g;/^$/d')

if [[ "$condition" == cold ]]; then
	rustup toolchain install "$toolchain" --profile "$profile" "${component_args[@]}"
fi

rustup default "$toolchain"
measurement_file="${RUNNER_TEMP:?}/codexy-rust-measurement/metrics/measurement.txt"
test -f "$measurement_file"
rustc_version="$(rustc --version)"
cargo_version="$(cargo --version)"
rust_host="$(rustc -vV | sed -n 's/^host: //p')"
printf 'toolchain=%s\nrustc=%s\ncargo=%s\nhost=%s\n' \
	"$toolchain" "$rustc_version" "$cargo_version" "$rust_host" >>"$measurement_file"
