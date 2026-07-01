#!/bin/sh
set -efu

fail() {
  printf '%s\n' "error: $1"
  exit 1
}

json_field_value() {
  json_text="$1"
  field_name="$2"
  field_rest=${json_text#*"\"$field_name\""}
  if [ "$field_rest" = "$json_text" ]; then
    printf '\n'
    return
  fi
  printf '%s\n' "${field_rest#*:}"
}

json_value_has_label_name() {
  field_value="$1"
  graph_key=$(printf '%s%s\n' "no" "des")
  case "$field_value" in
    \[*)
      field_items=$(printf '%s\n' "$field_value" | sed 's/^\[\([^]]*\)\].*/\1/')
      ;;
    \{*)
      field_items=$(printf '%s\n' "$field_value" |
        sed "s/^\\(.*\"$graph_key\"[[:space:]]*:[[:space:]]*\\[[^]]*\\]\\).*/\\1/")
      ;;
    *)
      return 1
      ;;
  esac
  case "$field_items" in
    *'"name"'*) return 0 ;;
    *) return 1 ;;
  esac
}

json_has_repository_label_taxonomy() {
  json_text="$1"
  graph_key=$(printf '%s%s\n' "no" "des")
  repository_labels=$(json_field_value "$json_text" "repositoryLabels")
  if json_value_has_label_name "$repository_labels"; then
    return 0
  fi
  case "$json_text" in
    *'"repository"'*'"labels"'*\""$graph_key"\"*'"name"'*) return 0 ;;
    *) return 1 ;;
  esac
}

json_has_pr_label_evidence() {
  json_text="$1"
  labels=$(json_field_value "$json_text" "labels")
  json_value_has_label_name "$labels"
}

pr_state_file="${1:-}"
[ -n "$pr_state_file" ] || fail "--pr-state-file is required"
[ -f "$pr_state_file" ] || fail "--pr-state-file must point to a readable file"
pr_state_json=$(tr -d '\n\r' < "$pr_state_file") || fail "could not read --pr-state-file"
if ! json_has_repository_label_taxonomy "$pr_state_json"; then
  exit 0
fi
json_has_pr_label_evidence "$pr_state_json" ||
  fail "PR labels missing label application evidence"
