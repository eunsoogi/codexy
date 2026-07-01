json_is_structurally_complete_object() {
  printf '%s\n' "$1" | awk '
function push(expected) {
  stack = stack expected
}
function pop(expected,    top) {
  if (length(stack) == 0) {
    valid = 0
    return
  }
  top = substr(stack, length(stack), 1)
  if (top != expected) {
    valid = 0
    return
  }
  stack = substr(stack, 1, length(stack) - 1)
}
function only_trailing_space(pos,    j) {
  for (j = pos; j <= length($0); j++) {
    if (substr($0, j, 1) !~ /[[:space:]]/) {
      return 0
    }
  }
  return 1
}
{
  valid = 1
  in_string = 0
  escape = 0
  stack = ""
  started = 0
  closed = 0
  for (i = 1; i <= length($0); i++) {
    c = substr($0, i, 1)
    if (closed) {
      if (!only_trailing_space(i)) {
        valid = 0
      }
      break
    }
    if (!started) {
      if (c ~ /[[:space:]]/) {
        continue
      }
      if (c != "{") {
        valid = 0
        break
      }
      started = 1
      push("}")
      continue
    }
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
      push("}")
    } else if (c == "[") {
      push("]")
    } else if (c == "}" || c == "]") {
      pop(c)
      if (!valid) {
        break
      }
      if (length(stack) == 0) {
        closed = 1
      }
    }
  }
  exit(valid && started && closed && !in_string && length(stack) == 0 ? 0 : 1)
}
'
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
