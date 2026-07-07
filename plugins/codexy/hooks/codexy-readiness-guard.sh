#!/bin/sh
set -efu

fail() {
  printf '%s\n' "error: $1"
  exit 1
}
is_ident() {
  case "$1" in
    "" | *[!abcdefghijklmnopqrstuvwxyz0123456789-]*) return 1 ;;
  esac
}
is_scope() {
  case "$1" in
    "" | *[!abcdefghijklmnopqrstuvwxyz0123456789_/-]*) return 1 ;;
  esac
}
check_conventional_subject() {
  subject="$1"
  case "$subject" in
    *": "*) ;; *) return 1 ;;
  esac
  prefix=${subject%%: *}
  summary=${subject#*: }
  case "$summary" in
    *[![:space:]]*) ;; *) return 1 ;;
  esac
  case "$prefix" in
    *!) prefix=${prefix%!} ;; *) ;;
  esac
  case "$prefix" in
    *"("*")")
      commit_type=${prefix%%(*}
      scope=${prefix#*(}
      scope=${scope%)}
      is_ident "$commit_type" && is_scope "$scope"
      ;;
    *) is_ident "$prefix" ;;
  esac
}
is_closing_keyword() {
  keyword=${1%:}
  keyword=$(printf '%s' "$keyword" | tr '[:upper:]' '[:lower:]')
  case "$keyword" in
    close | closes | closed | fix | fixes | fixed | resolve | resolves | resolved) return 0 ;;
    *) return 1 ;;
  esac
}
is_issue_number() {
  case "$1" in
    "" | *[!0123456789]*) return 1 ;;
  esac
}
is_owner_repo_reference() {
  case "$1" in
    */*) ;; *) return 1 ;;
  esac
  owner=${1%%/*}
  repo=${1#*/}
  case "$owner" in
    "" | *[!abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_.-]*) return 1 ;;
  esac
  case "$repo" in
    "" | *[!abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_.-]*) return 1 ;;
  esac
}

is_closing_issue_reference() {
  candidate="$1"
  while :; do
    case "$candidate" in
      *, | *.) candidate=${candidate%?} ;; *) break ;;
    esac
  done
  case "$candidate" in
    \#*)
      is_issue_number "${candidate#\#}"
      return
      ;;
    *\#*)
      owner_repo=${candidate%#*}
      issue=${candidate##*#}
      is_owner_repo_reference "$owner_repo" && is_issue_number "$issue"
      return
      ;;
    *) return 1 ;;
  esac
}

closing_reference_count() {
  count=0
  while [ "$#" -gt 0 ]; do
    token="$1"
    shift
    if ! is_closing_keyword "$token"; then
      continue
    fi
    for candidate in "$@"; do
      if is_closing_issue_reference "$candidate"; then
        count=$((count + 1))
        continue
      fi
      break
    done
  done
  printf '%s\n' "$count"
}

check_merge_message() {
  subject=${merge_message%%"
"*}
  expected_suffix=" (#$expected_pr)"
  case "$subject" in
    *"$expected_suffix") ;;
    *) fail "merge commit subject must end with the expected PR suffix: (#$expected_pr)" ;;
  esac
  subject=${subject%"$expected_suffix"}
  check_conventional_subject "$subject" ||
    fail "merge commit subject must use Conventional Commit style"

  old_ifs=$IFS
  IFS='
'
  set -- $merge_message
  IFS=$old_ifs
  closing_count=0
  last_non_empty=""
  for line do
    case "$line" in
      *[![:space:]]*) last_non_empty="$line" ;;
    esac
    set -- $line
    line_count=$(closing_reference_count "$@")
    closing_count=$((closing_count + line_count))
  done
  if [ -n "$expected_issue" ]; then
    expected_line="Fixes #$expected_issue"
    if [ "$closing_count" -ne 1 ] || [ "$last_non_empty" != "$expected_line" ]; then
      fail "merge commit message must contain exactly one closing reference, and the final closing line must be exactly: Fixes #$expected_issue"
    fi
  elif [ "$closing_count" -ne 0 ]; then
    fail "merge commit message must not contain closing references"
  fi
}

check_pr_labels() {
  [ -n "$pr_state_file" ] || fail "--pr-state-file is required"
  guard_dir=${0%/*}
  "$guard_dir/codexy-readiness-guard-pr-labels.sh" "$pr_state_file"
}
event="${1:-}"
case "$event" in
  UserPromptSubmit)
    printf '%s\n' '{"hookSpecificOutput":{"hookEventName":"UserPromptSubmit","additionalContext":"Codexy readiness guard: before creating a GitHub issue, run hooks/codexy-readiness-guard.sh --check-issue-title --issue-title \"ISSUE_TITLE\" with the exact issue title. Before PR readiness, run hooks/codexy-readiness-guard.sh --check-pr-title with the exact PR title and hooks/codexy-readiness-guard.sh --check-pr-labels --pr-state-file pr-state.json with captured repositoryLabels. Before merge readiness, run hooks/codexy-readiness-guard.sh --check-merge-message --expected-pr PR_NUMBER with the explicit squash merge message; for issue-backed PRs whose merge body must end in Fixes #ISSUE_NUMBER, add --expected-issue ISSUE_NUMBER."}}'
    exit 0
    ;;
esac

mode=""
pr_title=""
issue_title=""
expected_issue=""
expected_pr=""
merge_message=""
merge_message_file=""
pr_state_file=""

while [ "$#" -gt 0 ]; do
  case "$1" in
    --check-pr-title) mode="pr-title" ;;
    --check-issue-title) mode="issue-title" ;;
    --check-pr-labels) mode="pr-labels" ;;
    --check-merge-message) mode="merge-message" ;;
    --pr-title)
      [ "$#" -ge 2 ] || fail "--pr-title requires a value"
      shift; pr_title="$1"
      ;;
    --issue-title)
      [ "$#" -ge 2 ] || fail "--issue-title requires a value"
      shift; issue_title="$1"
      ;;
    --expected-pr)
      [ "$#" -ge 2 ] || fail "--expected-pr requires a value"
      shift; expected_pr="$1"
      ;;
    --expected-issue)
      [ "$#" -ge 2 ] || fail "--expected-issue requires a value"
      shift; expected_issue="$1"
      ;;
    --merge-message)
      [ "$#" -ge 2 ] || fail "--merge-message requires a value"
      shift; merge_message="$1"
      ;;
    --merge-message-file)
      [ "$#" -ge 2 ] || fail "--merge-message-file requires a value"
      shift; merge_message_file="$1"
      ;;
    --pr-state-file)
      [ "$#" -ge 2 ] || fail "--pr-state-file requires a value"
      shift; pr_state_file="$1"
      ;;
    *)
      fail "unknown argument: $1"
      ;;
  esac
  shift
done

case "$mode" in
pr-title)
    [ -n "$pr_title" ] || fail "--pr-title is required"
    check_conventional_subject "$pr_title" || fail "PR title must use Conventional Commit style"
    ;;
  issue-title)
    [ -n "$issue_title" ] || fail "--issue-title is required"
    lc_issue_title=$(printf '%s' "$issue_title" | tr '[:upper:]' '[:lower:]')
    check_conventional_subject "$lc_issue_title" && fail "issue title must not use Conventional Commit style"
    issue_prefix=$(printf '%s\n' "$lc_issue_title" | awk '{ print $1; exit }')
    issue_token=$issue_prefix
    case "$issue_prefix" in *:*) issue_prefix=${issue_prefix%%:*} ;; esac; while :; do case "$issue_prefix" in *:) issue_prefix=${issue_prefix%:} ;; *) break ;; esac; done
    case "$issue_token" in *"("*")" | *! | *:*) check_conventional_subject "$issue_prefix: issue" && fail "issue title must not use Conventional Commit style" ;; esac
    case "$issue_title" in [ABCDEFGHIJKLMNOPQRSTUVWXYZ]*) ;; *) fail "issue title must start with an uppercase descriptive title" ;; esac
    ;;
  pr-labels)
    check_pr_labels
    ;;
  merge-message)
    [ -n "$expected_pr" ] || fail "--expected-pr is required"
    case "$expected_pr" in
      "" | *[!0123456789]*) fail "--expected-pr must be numeric" ;;
    esac
    case "$expected_issue" in
      "" | *[!0123456789]*)
        if [ -n "$expected_issue" ]; then
          fail "--expected-issue must be numeric"
        fi
        ;;
    esac
    if [ -n "$merge_message_file" ]; then
      merge_message=$(cat "$merge_message_file")
    fi
    [ -n "$merge_message" ] || fail "--merge-message or --merge-message-file is required"
    check_merge_message
    ;;
  *)
    fail "--check-issue-title, --check-pr-title, --check-pr-labels, or --check-merge-message is required"
    ;;
esac

printf '%s\n' "codexy readiness guard ok"
