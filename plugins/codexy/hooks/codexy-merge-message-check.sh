#!/bin/sh
set -efu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)

case "${1:-}" in
  UserPromptSubmit)
    printf '%s\n' '{"hookSpecificOutput":{"hookEventName":"UserPromptSubmit","additionalContext":"Codexy hard merge-message check: before merge readiness, run hooks/codexy-merge-message-check.sh --expected-pr PR_NUMBER with the explicit squash merge message; for issue-backed PRs, add --expected-issue ISSUE_NUMBER. context-only hooks do not enforce merge-message validity."}}'
    exit 0
    ;;
esac

"$script_dir/codexy-readiness-guard.sh" --check-merge-message "$@"
