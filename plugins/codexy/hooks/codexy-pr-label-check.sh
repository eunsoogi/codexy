#!/bin/sh
set -efu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)

case "${1:-}" in
  UserPromptSubmit)
    printf '%s\n' '{"hookSpecificOutput":{"hookEventName":"UserPromptSubmit","additionalContext":"Codexy hard PR label check: before PR readiness, capture PR state with repositoryLabels and run hooks/codexy-pr-label-check.sh --pr-state-file pr-state.json, or the equivalent completion-handoff validator path. context-only hooks do not enforce label application."}}'
    exit 0
    ;;
esac

"$script_dir/codexy-readiness-guard.sh" --check-pr-labels "$@"
