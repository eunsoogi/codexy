#!/bin/sh
# Native Codex plugin hook. Codex resolves PLUGIN_ROOT before launching this file.
set -efu

[ -n "${PLUGIN_ROOT-}" ] || exit 0
payload=$(/bin/cat)

case "$payload" in
*[Gg][Ii][Tt][Hh][Uu][Bb]* | *[Ii][Ss][Ss][Uu][Ee]* | *[Pp][Uu][Ll][Ll]\ [Rr][Ee][Qq][Uu][Ee][Ss][Tt]* | *[Pp][Uu][Ll][Ll][Rr][Ee][Qq][Uu][Ee][Ss][Tt]* | *[Rr][Ee][Vv][Ii][Ee][Ww]* | *[Mm][Ee][Rr][Gg][Ee]*)
	# shellcheck disable=SC2016 # $git-workflow is literal prompt text.
	printf '%s\n' '{"hookSpecificOutput":{"hookEventName":"UserPromptSubmit","additionalContext":"Codexy GitHub workflow is installed. Use $git-workflow; its package-owned generic admission hooks are active."}}'
	;;
esac
