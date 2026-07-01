json_is_structurally_complete_object() {
  command -v jq >/dev/null 2>&1 || return 1
  printf '%s\n' "$1" | jq -e 'type == "object"' >/dev/null 2>&1
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

top_level_json_object_field_value() {
  json_text="$1"
  field_name="$2"
  printf '%s\n' "$json_text" | awk -v key="$field_name" '
function skip_spaces(pos) {
  while (substr($0, pos, 1) ~ /[[:space:]]/) {
    pos++
  }
  return pos
}
function emit_object(start,    i, c, depth, in_string, escape) {
  depth = 0
  in_string = 0
  escape = 0
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
    } else if (c == "{") {
      depth++
    } else if (c == "}") {
      depth--
      if (depth == 0) {
        print substr($0, start, i - start + 1)
        exit
      }
    }
  }
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
        value_start = skip_spaces(after_key + 1)
        if (substr($0, value_start, 1) == "{") {
          emit_object(value_start)
        }
        i = after_key
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

json_string_field_value() {
  value=$(top_level_json_field_value "$1" "$2")
  case "$value" in
    \"*) printf '%s\n' "$value" | sed 's/^[[:space:]]*"\([^"]*\)".*/\1/' | tr '[:upper:]' '[:lower:]' ;;
    *) printf '\n' ;;
  esac
}
