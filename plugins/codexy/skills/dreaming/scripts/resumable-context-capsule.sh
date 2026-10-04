#!/bin/sh
set -eu
script_dir=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
# Keep capsule checks in Python, which validates the capsule against a separate authority document.
exec python3 "$script_dir/resumable_context_capsule.py" "$@"
