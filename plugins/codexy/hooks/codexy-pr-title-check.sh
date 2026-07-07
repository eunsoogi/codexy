#!/bin/sh
set -efu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)

case "${1:-}" in
  UserPromptSubmit)
    printf '%s\n' '{"hookSpecificOutput":{"hookEventName":"UserPromptSubmit","additionalContext":"Codexy hard PR title check: before PR readiness, run hooks/codexy-pr-title-check.sh --pr-title with the exact GitHub PR title, or the equivalent scripts/validate-plugin-config --check-pr-title command. context-only hooks do not enforce PR title validity."}}'
    exit 0
    ;;
esac

"$script_dir/codexy-readiness-guard.sh" --check-pr-title "$@"
