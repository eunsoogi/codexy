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

top_level_json_field_value() {
  json_text="$1"
  field_name="$2"
  printf '%s\n' "$json_text" | awk -v key="$field_name" '
function skip_spaces(pos) {
  while (substr($0, pos, 1) ~ /[[:space:]]/) {
    pos++
  }
  return pos
}
function emit_value(start,    i, c, depth, in_string, escape, seen) {
  depth = 0
  in_string = 0
  escape = 0
  seen = 0
  for (i = start; i <= length($0); i++) {
    c = substr($0, i, 1)
    if (in_string) {
      if (escape) {
        escape = 0
      } else if (c == "\\") {
        escape = 1
      } else if (c == "\"") {
        in_string = 0
      }
    } else if (c == "\"") {
      in_string = 1
      seen = 1
    } else if (c == "{" || c == "[") {
      depth++
      seen = 1
    } else if (c == "}" || c == "]") {
      if (depth == 0) {
        print substr($0, start, i - start)
        exit
      }
      depth--
      if (depth == 0) {
        print substr($0, start, i - start + 1)
        exit
      }
    } else if (c == "," && depth == 0 && seen) {
      print substr($0, start, i - start)
      exit
    } else if (c !~ /[[:space:]]/) {
      seen = 1
    }
  }
  print substr($0, start)
}
{
  pattern = "\"" key "\""
  depth = 0
  in_string = 0
  escape = 0
  for (i = 1; i <= length($0); i++) {
    c = substr($0, i, 1)
    if (in_string) {
      if (escape) {
        escape = 0
      } else if (c == "\\") {
        escape = 1
      } else if (c == "\"") {
        in_string = 0
      }
    } else if (depth == 1 && substr($0, i, length(pattern)) == pattern) {
      after_key = skip_spaces(i + length(pattern))
      if (substr($0, after_key, 1) == ":") {
        emit_value(skip_spaces(after_key + 1))
      }
    } else if (c == "\"") {
      in_string = 1
    } else if (c == "{" || c == "[") {
      depth++
    } else if (c == "}" || c == "]") {
      depth--
    }
  }
}
'
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
  repository_labels=$(top_level_json_field_value "$json_text" "repositoryLabels")
  if json_value_has_label_name "$repository_labels"; then
    return 0
  fi
  repository=$(top_level_json_field_value "$json_text" "repository")
  repository_labels=$(top_level_json_field_value "$repository" "labels")
  json_value_has_label_name "$repository_labels"
}

json_has_pr_label_evidence() {
  json_text="$1"
  labels=$(top_level_json_field_value "$json_text" "labels")
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
