json_is_structurally_complete_object() {
  printf '%s\n' "$1" | awk '
function ws() { while (i <= n && substr(s, i, 1) ~ /[[:space:]]/) i++ }
function str(    c, e, h) {
  if (substr(s, i, 1) != "\"") return 0
  i++
  while (i <= n) {
    c = substr(s, i, 1)
    if (c == "\\") {
      e = substr(s, i + 1, 1)
      if (e ~ /["\\\/bfnrt]/) i += 2
      else if (e == "u") {
        h = substr(s, i + 2, 4)
        if (length(h) != 4 || h !~ /^[0-9A-Fa-f][0-9A-Fa-f][0-9A-Fa-f][0-9A-Fa-f]$/) return 0
        i += 6
      } else return 0
    } else if (c == "\"") {
      i++
      return 1
    } else {
      i++
    }
  }
  return 0
}
function lit(t) { if (substr(s, i, length(t)) != t) return 0; i += length(t); return 1 }
function num(    c) {
  if (substr(s, i, 1) == "-") i++
  c = substr(s, i, 1)
  if (c == "0") {
    i++
    if (substr(s, i, 1) ~ /[0-9]/) return 0
  } else if (c ~ /[1-9]/) {
    while (i <= n && substr(s, i, 1) ~ /[0-9]/) i++
  } else return 0
  if (substr(s, i, 1) == ".") {
    i++
    if (!(substr(s, i, 1) ~ /[0-9]/)) return 0
    while (i <= n && substr(s, i, 1) ~ /[0-9]/) i++
  }
  if (substr(s, i, 1) ~ /[eE]/) {
    i++
    if (substr(s, i, 1) ~ /[+-]/) i++
    if (!(substr(s, i, 1) ~ /[0-9]/)) return 0
    while (i <= n && substr(s, i, 1) ~ /[0-9]/) i++
  }
  return 1
}
function val(    c) {
  ws()
  c = substr(s, i, 1)
  if (c == "{") return obj()
  if (c == "[") return arr()
  if (c == "\"") return str()
  if (c == "t") return lit("true")
  if (c == "f") return lit("false")
  if (c == "n") return lit("null")
  return num()
}
function arr() {
  i++
  ws()
  if (substr(s, i, 1) == "]") { i++; return 1 }
  while (val()) {
    ws()
    if (substr(s, i, 1) == "]") { i++; return 1 }
    if (substr(s, i, 1) != ",") return 0
    i++
  }
  return 0
}
function obj() {
  i++
  ws()
  if (substr(s, i, 1) == "}") { i++; return 1 }
  while (str()) {
    ws()
    if (substr(s, i, 1) != ":") return 0
    i++
    if (!val()) return 0
    ws()
    if (substr(s, i, 1) == "}") { i++; return 1 }
    if (substr(s, i, 1) != ",") return 0
    i++
    ws()
  }
  return 0
}
{ s = $0 }
END {
  n = length(s); i = 1; ws()
  if (substr(s, i, 1) != "{") exit 1
  if (!obj()) exit 1
  ws()
  exit(i > n ? 0 : 1)
}
'
}
top_level_json_field_value() {
  json_text="$1"; field_name="$2"
  printf '%s\n' "$json_text" | awk -v key="$field_name" '
function skip_spaces(pos) { while (substr($0, pos, 1) ~ /[[:space:]]/) pos++; return pos }
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
  json_text="$1"; field_name="$2"
  printf '%s\n' "$json_text" | awk -v key="$field_name" '
function skip_spaces(pos) { while (substr($0, pos, 1) ~ /[[:space:]]/) pos++; return pos }
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
