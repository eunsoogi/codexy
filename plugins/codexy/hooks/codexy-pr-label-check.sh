#!/bin/sh
set -efu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)

"$script_dir/codexy-readiness-guard.sh" --check-pr-labels "$@"
