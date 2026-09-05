#!/bin/sh

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

has_invalid_title_character() {
	case "$1" in
	*[[:cntrl:]]* | *' '* | *' '*) return 0 ;;
	*) return 1 ;;
	esac
}

has_terminal_reference() {
	printf '%s\n' "$1" | awk '
function trim_terminal_whitespace(value) {
	sub(/[ \t]+$/, "", value)
	return value
}
function strip_punctuation(value) {
	value = trim_terminal_whitespace(value)
	while (value ~ /[.,]$/) sub(/[.,]$/, "", value)
	value = trim_terminal_whitespace(value)
	return value
}
{
	value = strip_punctuation($0)
	lower = tolower(value)
	if (lower ~ /(^|[ \t])#[0-9]+$/ ||
	    lower ~ /(^|[ \t])\(#[0-9]+\)$/ ||
	    lower ~ /(^|[ \t])\[#[0-9]+\]$/ ||
	    lower ~ /(^|[ \t])\((pr|issue)[ \t]+#[0-9]+\)$/ ||
	    lower ~ /(^|[ \t])(pr|issue)[ \t]+#[0-9]+$/) exit 0
	exit 1
}'
}

check_conventional_subject() {
	subject=$1
	has_invalid_title_character "$subject" && return 1
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
	*) ;;
	esac
	case "$prefix" in
	*"("*")") ;;
	*) return 1 ;;
	esac
	commit_type=${prefix%%(*}
	scope=${prefix#*(}
	scope=${scope%)}
	is_ident "$commit_type" && is_scope "$scope" || return 1
	has_terminal_reference "$summary" && return 1
	return 0
}

is_label_separator() {
	case "$1" in
	":"* | "："*) return 0 ;;
	-" "* | "– "* | "— "*) return 0 ;;
	*) return 1 ;;
	esac
}

is_issue_category() {
	normalized=$(printf '%s\n' "$1" | sed 's/：/:/g; s/–/-/g; s/—/-/g')
	printf '%s\n' "$normalized" | awk '
function valid_type(value) { return value ~ /^[a-z0-9-]+$/ }
function valid_scope(value) { return value ~ /^[a-z0-9_\/-]+$/ }
function spaces(value, position, size) {
	size = length(value)
	while (position <= size && substr(value, position, 1) ~ /[ \t]/) position++
	return position
}
function dash_separator(value) { return value ~ /^[-–—]($|[ \t])/ }
function complete_prefix(value,    lower) {
	lower = tolower(value)
	return lower ~ /^[a-z0-9-]+([ \t]*\([ \t]*[a-z0-9_\/-]+[ \t]*\))?[ \t]*!?$/
}
function category(value,    closing, inner, size, position, type_end, after, scope_start, scope, scoped, breaking, rest, trimmed) {
	if (substr(value, 1, 1) == "[") {
		closing = index(value, "]")
		if (closing > 2) {
			inner = substr(value, 2, closing - 2)
			if (complete_prefix(inner)) return 1
		}
	}
	size = length(value)
	position = 1
	while (position <= size && substr(value, position, 1) ~ /[A-Za-z0-9-]/) position++
	if (!valid_type(tolower(substr(value, 1, position - 1)))) return 0
	type_end = position
	after = spaces(value, type_end)
	scoped = 0
	position = type_end
	if (substr(value, after, 1) == "(") {
		scoped = 1
		position = spaces(value, after + 1)
		scope_start = position
		while (position <= size && substr(value, position, 1) ~ /[A-Za-z0-9_\/-]/) position++
		if (scope_start == position || !valid_scope(tolower(substr(value, scope_start, position - scope_start)))) return 0
		position = spaces(value, position)
		if (substr(value, position, 1) != ")") return 0
		position++
	}
	breaking = 0
	after = spaces(value, position)
	if (substr(value, after, 1) == "!") {
		breaking = 1
		position = spaces(value, after + 1)
	}
	rest = substr(value, position)
	if (scoped || breaking) return rest == "" || rest ~ /^[ \t]/ || rest ~ /^[:：]/ || dash_separator(rest)
	if (rest == "" || rest ~ /^[ \t]*$/) return 1
	trimmed = rest
	sub(/^[ \t]*/, "", trimmed)
	return trimmed ~ /^[:：]/ || dash_separator(trimmed)
}
{ exit(category($0) ? 0 : 1) }
'
}

check_issue_title() {
	title=$1
	case "$title" in
	[ABCDEFGHIJKLMNOPQRSTUVWXYZ]*) ;;
	*) return 1 ;;
	esac
	has_invalid_title_character "$title" && return 1
	is_issue_category "$title" && return 1
	return 0
}
