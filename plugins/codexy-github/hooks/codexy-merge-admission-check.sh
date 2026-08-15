#!/bin/sh
# Validate both independent merge admissions before a documented squash command.
set -efu

script_dir=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
expected_pr=
expected_issue=
message_file=
authorization_file=
pr_state_file=

while [ "$#" -gt 0 ]; do
	case "$1" in
	--expected-pr)
		expected_pr=${2-}
		shift 2
		;;
	--expected-issue)
		expected_issue=${2-}
		shift 2
		;;
	--merge-message-file)
		message_file=${2-}
		shift 2
		;;
	--merge-authorization-file)
		authorization_file=${2-}
		shift 2
		;;
	--merge-authorization-pr-state-file)
		pr_state_file=${2-}
		shift 2
		;;
	*)
		printf '%s\n' "unsupported merge admission argument: $1" >&2
		exit 2
		;;
	esac
done

for required in "$expected_pr" "$message_file" "$authorization_file" "$pr_state_file"; do
	[ -n "$required" ] || {
		printf '%s\n' 'missing required merge admission argument' >&2
		exit 2
	}
done

set -- --expected-pr "$expected_pr" --merge-message-file "$message_file"
[ -z "$expected_issue" ] || set -- "$@" --expected-issue "$expected_issue"
"$script_dir/codexy-merge-message-check.sh" "$@"
python3 -I -B "$script_dir/codexy-merge-authorization-check.py" \
	--authorization-file "$authorization_file" \
	--pr-state-file "$pr_state_file"
