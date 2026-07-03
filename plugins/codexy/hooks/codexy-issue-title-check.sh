#!/bin/sh
set -efu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)

case "${1:-}" in
  UserPromptSubmit)
    printf '%s\n' '{"hookSpecificOutput":{"hookEventName":"UserPromptSubmit","additionalContext":"Codexy hard issue title check: before creating a GitHub issue, run hooks/codexy-issue-title-check.sh --issue-title with the exact issue title, or the equivalent scripts/validate-plugin-config --check-issue-title command. Issue titles MUST start with uppercase descriptive prose and MUST NOT use Conventional Commit prefixes."}}'
    exit 0
    ;;
esac

"$script_dir/codexy-readiness-guard.sh" --check-issue-title "$@"
