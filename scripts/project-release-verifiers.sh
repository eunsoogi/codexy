#!/bin/sh
set -eu

activation_commit=${1:?activation commit required}
for commit in "$GITHUB_SHA" "$activation_commit"; do
	case "$commit" in *[!0-9a-f]* | '') exit 1 ;; esac
	test "${#commit}" -eq 40
done
test "$GITHUB_REF" = refs/heads/main
test "$GITHUB_SHA" = "$(git rev-parse origin/main)"
test "$(git rev-parse "$activation_commit")" = "$activation_commit"
git checkout --detach "$activation_commit"
test "$(git rev-parse HEAD)" = "$activation_commit"

actual_paths="$(git diff --name-only "$activation_commit" "$GITHUB_SHA" -- scripts | sort)"
if test -n "$actual_paths"; then
	while IFS= read -r path; do
		case "$path" in
		scripts/project-release-verifiers.sh) ;;
		scripts/verify-release-attestation-set | scripts/verify-release-attestation-total)
			git checkout "$GITHUB_SHA" -- "$path"
			;;
		*)
			exit 1
			;;
		esac
	done <<EOF
$actual_paths
EOF
fi
for verifier in scripts/verify-release-attestation-set scripts/verify-release-attestation-total; do
	test -x "$verifier"
	test "$(git hash-object "$verifier")" = "$(git rev-parse "$GITHUB_SHA:$verifier")"
done
test "$(git rev-parse HEAD)" = "$activation_commit"
