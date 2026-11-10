#!/bin/sh
# Recognize either complete capsule source layout before staging runtime bytes.
set -eu

schema="packages/codexy-runtime/schemas/handoff-runtime.schema.json"
current="plugins/codexy/skills/self-improvement/scripts/resumable-context-capsule.sh plugins/codexy/skills/self-improvement/scripts/resumable-context-capsule.cmd plugins/codexy/skills/self-improvement/scripts/resumable_context_capsule.py"
previous="plugins/codexy/skills/dreaming/scripts/resumable-context-capsule.sh plugins/codexy/skills/dreaming/scripts/resumable-context-capsule.cmd plugins/codexy/skills/dreaming/scripts/resumable_context_capsule.py"
schema_count=0 current_count=0 previous_count=0
test ! -f "$schema" || schema_count=1
for source in $current; do test ! -f "$source" || current_count=$((current_count + 1)); done
for source in $previous; do test ! -f "$source" || previous_count=$((previous_count + 1)); done
case "$schema_count:$current_count:$previous_count" in
0:0:0) printf '0\n' ;;
1:3:0|1:0:3) printf '1\n' ;;
*) echo "core-handoff source inventory is partial or mixed" >&2; exit 1 ;;
esac
