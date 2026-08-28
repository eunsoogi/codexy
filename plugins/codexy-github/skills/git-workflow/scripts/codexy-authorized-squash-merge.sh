#!/bin/sh
# Host-resolved skill entrypoint for the canonical hooked merge wrapper.
set -eu

script_dir=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd -P)
wrapper="$script_dir/../../../hooks/codexy-authorized-squash-merge.sh"
[ -x "$wrapper" ] || {
	printf '%s\n' 'canonical authorized squash merge wrapper is unavailable' >&2
	exit 2
}
exec "$wrapper" "$@"
