#!/bin/sh
set -efu

script_dir=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)

# Keep merge-message policy in the shared guard and select only its merge-message mode here.
"$script_dir/codexy-readiness-guard.sh" --check-merge-message "$@"
