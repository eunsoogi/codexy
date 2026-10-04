# shellcheck shell=sh
# Read quoted scalar fields used by readiness checks and normalize case and slash escapes for comparison.
json_string_field_value() {
	value=$(top_level_json_field_value "$1" "$2")
	case "$value" in
	\"*) printf '%s\n' "$value" | sed 's/^[[:space:]]*"\([^"]*\)".*/\1/; s#\\/#/#g; s#\\u002[fF]#/#g' | tr '[:upper:]' '[:lower:]' ;;
	*) printf '\n' ;;
	esac
}
