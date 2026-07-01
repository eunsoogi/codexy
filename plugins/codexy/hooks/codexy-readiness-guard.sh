#!/bin/sh
set -eu

fail() {
  printf '%s\n' "error: $1"
  exit 1
}

is_ident() {
  case "$1" in
    "" | *[!abcdefghijklmnopqrstuvwxyz0123456789-]*)
      return 1
      ;;
  esac
}

is_scope() {
  case "$1" in
    "" | *[!abcdefghijklmnopqrstuvwxyz0123456789_/-]*)
      return 1
      ;;
  esac
}

check_conventional_subject() {
  subject="$1"
  case "$subject" in
    *": "*) ;;
    *) return 1 ;;
  esac
  prefix=${subject%%: *}
  summary=${subject#*: }
  case "$summary" in
    *[![:space:]]*) ;;
    *) return 1 ;;
  esac
  case "$prefix" in
    *!) prefix=${prefix%!} ;;
  esac
  case "$prefix" in
    *"("*")")
      commit_type=${prefix%%(*}
      scope=${prefix#*(}
      scope=${scope%)}
      is_ident "$commit_type" && is_scope "$scope"
      ;;
    *)
      is_ident "$prefix"
      ;;
  esac
}

event="${1:-}"
case "$event" in
  UserPromptSubmit)
    cat <<JSON
{"hookSpecificOutput":{"hookEventName":"UserPromptSubmit","additionalContext":"Codexy readiness guard: before PR readiness, run hooks/codexy-readiness-guard.sh --check-pr-title with the exact PR title. Before merge readiness, run hooks/codexy-readiness-guard.sh --check-merge-message with the explicit squash merge message and expected PR number."}}
JSON
    exit 0
    ;;
esac

mode=""
pr_title=""
expected_pr=""
merge_message=""
merge_message_file=""

while [ "$#" -gt 0 ]; do
  case "$1" in
    --check-pr-title)
      mode="pr-title"
      ;;
    --check-merge-message)
      mode="merge-message"
      ;;
    --pr-title)
      [ "$#" -ge 2 ] || fail "--pr-title requires a value"
      shift
      pr_title="$1"
      ;;
    --expected-pr)
      [ "$#" -ge 2 ] || fail "--expected-pr requires a value"
      shift
      expected_pr="$1"
      ;;
    --merge-message)
      [ "$#" -ge 2 ] || fail "--merge-message requires a value"
      shift
      merge_message="$1"
      ;;
    --merge-message-file)
      [ "$#" -ge 2 ] || fail "--merge-message-file requires a value"
      shift
      merge_message_file="$1"
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
  merge-message)
    [ -n "$expected_pr" ] || fail "--expected-pr is required"
    case "$expected_pr" in
      "" | *[!0123456789]*)
        fail "--expected-pr must be numeric"
        ;;
    esac
    if [ -n "$merge_message_file" ]; then
      merge_message=$(cat "$merge_message_file")
    fi
    [ -n "$merge_message" ] || fail "--merge-message or --merge-message-file is required"
    subject=$(printf '%s\n' "$merge_message" | sed -n '1p')
    expected_suffix=" (#$expected_pr)"
    case "$subject" in
      *"$expected_suffix") ;;
      *) fail "merge commit subject must end with the expected PR suffix: (#$expected_pr)" ;;
    esac
    subject=${subject%"$expected_suffix"}
    check_conventional_subject "$subject" ||
      fail "merge commit subject must use Conventional Commit style"
    ;;
  *)
    fail "--check-pr-title or --check-merge-message is required"
    ;;
esac

printf '%s\n' "codexy readiness guard ok"
