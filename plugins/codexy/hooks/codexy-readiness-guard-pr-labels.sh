#!/bin/sh
set -efu

fail() {
  printf '%s\n' "error: $1"
  exit 1
}

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
. "$script_dir/codexy-readiness-guard-json.sh"

json_is_open_pr() { [ "$(json_string_field_value "$1" "state")" = "open" ]; }

json_is_codexy_lane() {
  json_text="$1"
  for field_name in repository nameWithOwner headRepository; do
    [ "$(json_string_field_value "$json_text" "$field_name")" = "eunsoogi/codexy" ] && return 0
  done
  url=$(json_string_field_value "$json_text" "url")
  case "$url" in
    *github.com/eunsoogi/codexy/* | *github.com/eunsoogi/codexy) return 0 ;;
  esac
  return 1
}

json_has_pr_identity() {
  json_text="$1"
  for field_name in repository nameWithOwner headRepository url; do
    [ -n "$(json_string_field_value "$json_text" "$field_name")" ] && return 0
  done
  return 1
}

json_value_has_label_name() {
  field_value="$1"
  graph_key=$(printf '%s%s\n' "no" "des")
  value_is_array=0
  case "$field_value" in
    \[*)
      value_is_array=1
      field_items=$(printf '%s\n' "$field_value" | sed 's/^\[\([^]]*\)\].*/\1/')
      ;;
    \{*)
      field_items=$(printf '%s\n' "$field_value" |
        sed "s/^.*\"$graph_key\"[[:space:]]*:[[:space:]]*\\[\\([^]]*\\)\\].*/\\1/")
      if [ "$field_items" = "$field_value" ]; then
        return 1
      fi
      value_is_array=1
      ;;
    *)
      return 1
      ;;
  esac
  if printf '%s\n' "$field_items" | grep -Eq '"name"[[:space:]]*:[[:space:]]*"[^"]+"'; then
    return 0
  fi
  if [ "$value_is_array" -eq 1 ] &&
    printf '%s\n' "$field_items" | grep -Eq '(^|,)[[:space:]]*"[^"]+"'; then
    return 0
  fi
  return 1
}

json_value_is_label_taxonomy_capture() {
  case "$1" in
    \[*)
      printf '%s\n' "$1" | grep -Eq '^\[[[:space:]]*\]$'
      ;;
    \{*)
      printf '%s\n' "$1" | grep -Eq '"nodes"[[:space:]]*:[[:space:]]*\[[[:space:]]*\]'
      ;;
    *)
      return 1
      ;;
  esac
}

json_has_repository_label_taxonomy() {
  json_text="$1"
  found_empty_taxonomy=0
  repository_labels=$(top_level_json_field_value "$json_text" "repositoryLabels")
  if json_value_has_label_name "$repository_labels"; then
    return 0
  fi
  if json_value_is_label_taxonomy_capture "$repository_labels"; then
    found_empty_taxonomy=1
  fi
  repository=$(top_level_json_object_field_value "$json_text" "repository")
  repository_labels=$(top_level_json_field_value "$repository" "labels")
  if json_value_has_label_name "$repository_labels"; then
    return 0
  fi
  if json_value_is_label_taxonomy_capture "$repository_labels"; then
    found_empty_taxonomy=1
  fi
  [ "$found_empty_taxonomy" -eq 1 ] && return 2
  return 1
}

json_has_pr_label_evidence() {
  json_text="$1"
  labels=$(top_level_json_field_value "$json_text" "labels")
  json_value_has_label_name "$labels"
}

pr_state_file="${1:-}"
[ -n "$pr_state_file" ] || fail "--pr-state-file is required"
[ -f "$pr_state_file" ] || fail "--pr-state-file must point to a readable file"
pr_state_json=$(cat "$pr_state_file") || fail "could not read --pr-state-file"
json_is_structurally_complete_object "$pr_state_json" ||
  fail "PR state malformed JSON evidence"
pr_state_json=$(printf '%s' "$pr_state_json" | tr -d '\n\r')
pr_state_state=$(json_string_field_value "$pr_state_json" "state")
[ -n "$pr_state_state" ] || fail "PR state missing state evidence"
json_has_pr_identity "$pr_state_json" || fail "PR state missing repository identity evidence"
if ! json_is_open_pr "$pr_state_json" || ! json_is_codexy_lane "$pr_state_json"; then
  exit 0
fi
if json_has_repository_label_taxonomy "$pr_state_json"; then
  :
else
  taxonomy_status=$?
  [ "$taxonomy_status" -eq 2 ] && exit 0
  fail "GitHub label evidence missing repositoryLabels taxonomy"
fi
json_has_pr_label_evidence "$pr_state_json" ||
  fail "PR labels missing label application evidence"
