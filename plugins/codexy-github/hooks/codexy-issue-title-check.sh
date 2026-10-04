#!/bin/sh
set -efu

script_dir=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)

# Keep issue-title policy in the shared guard and select only its issue-title mode here.
"$script_dir/codexy-readiness-guard.sh" --check-issue-title "$@"
