#!/bin/sh
# Admit the exact squash merge, then execute it through the installed hook path.
set -efu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
expected_pr=
expected_issue=
message_file=
authorization_file=
pr_state_file=
repo=
head_oid=
subject=
body_file=

while [ "$#" -gt 0 ]; do
  case "$1" in
    --expected-pr) expected_pr=${2-}; shift 2 ;;
    --expected-issue) expected_issue=${2-}; shift 2 ;;
    --merge-message-file) message_file=${2-}; shift 2 ;;
    --merge-authorization-file) authorization_file=${2-}; shift 2 ;;
    --merge-authorization-pr-state-file) pr_state_file=${2-}; shift 2 ;;
    --repo) repo=${2-}; shift 2 ;;
    --match-head-commit) head_oid=${2-}; shift 2 ;;
    --subject) subject=${2-}; shift 2 ;;
    --body-file) body_file=${2-}; shift 2 ;;
    *) printf '%s\n' "unsupported authorized merge argument: $1" >&2; exit 2 ;;
  esac
done

for required in "$expected_pr" "$message_file" "$authorization_file" "$pr_state_file" \
  "$repo" "$head_oid" "$subject" "$body_file"; do
  [ -n "$required" ] || { printf '%s\n' 'missing required authorized merge argument' >&2; exit 2; }
done

set -- --expected-pr "$expected_pr" --merge-message-file "$message_file"
[ -z "$expected_issue" ] || set -- "$@" --expected-issue "$expected_issue"
"$script_dir/codexy-merge-admission-check.sh" "$@" \
  --merge-authorization-file "$authorization_file" \
  --merge-authorization-pr-state-file "$pr_state_file"
exec gh pr merge "$expected_pr" --repo "$repo" --squash --delete-branch \
  --match-head-commit "$head_oid" --subject "$subject" --body-file "$body_file"
