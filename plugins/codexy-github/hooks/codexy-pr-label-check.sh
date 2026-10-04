#!/bin/sh
set -efu

script_dir=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)

# Keep PR-label policy in the shared guard and select only its label-validation mode here.
"$script_dir/codexy-readiness-guard.sh" --check-pr-labels "$@"
