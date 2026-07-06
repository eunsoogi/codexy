#!/bin/sh
set -efu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
repo_root=$(CDPATH= cd -- "$script_dir/../../.." && pwd)

case "${1:-}" in
  UserPromptSubmit)
    printf '%s\n' '{"hookSpecificOutput":{"hookEventName":"UserPromptSubmit","additionalContext":"Codexy hard issue title check: before creating a GitHub issue, run hooks/codexy-issue-title-check.sh --issue-title \"ISSUE_TITLE\" with the exact issue title, or the equivalent scripts/validate-plugin-config --check-issue-title --issue-title \"ISSUE_TITLE\" command. Issue titles MUST start with uppercase descriptive prose and MUST NOT use Conventional Commit prefixes."}}'
    exit 0
    ;;
esac

"$repo_root/scripts/validate-plugin-config" --check-issue-title "$@"
