#!/bin/sh
set -efu

script_dir=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)

# Keep PR-title policy in the shared guard and select only its PR-title mode here.
"$script_dir/codexy-readiness-guard.sh" --check-pr-title "$@"
