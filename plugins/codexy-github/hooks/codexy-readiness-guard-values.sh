# shellcheck shell=sh
json_string_field_value() {
	value=$(top_level_json_field_value "$1" "$2")
	case "$value" in
	\"*) printf '%s\n' "$value" | sed 's/^[[:space:]]*"\([^"]*\)".*/\1/; s#\\/#/#g; s#\\u002[fF]#/#g' | tr '[:upper:]' '[:lower:]' ;;
	*) printf '\n' ;;
	esac
}
