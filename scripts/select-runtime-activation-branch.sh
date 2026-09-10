#!/bin/sh
set -eu

version=${1:?bootstrap version is required}
receipt=${2:?candidate receipt is required}
repository=${GITHUB_REPOSITORY:?repository is required}

fail() {
	printf '%s\n' "$1" >&2
	exit 1
}

test -f "$receipt" && test ! -L "$receipt" || fail "candidate receipt must be a regular file"
version_ok="$(printf '%s\n' "$version" | awk -F. 'NF == 3 && $1 != "" && $2 != "" && $3 != "" && $1 ~ /^[0-9]+$/ && $2 ~ /^[0-9]+$/ && $3 ~ /^[0-9]+$/ { print "ok" }')"
test "$version_ok" = ok || fail "bootstrap version is not MAJOR.MINOR.PATCH"

schema="$(jq -er '.schema' "$receipt")" || fail "candidate receipt schema is unreadable"
test "$schema" = codexy-runtime-candidate-receipt/v1 || fail "candidate receipt schema is not canonical"
source_commit="$(jq -er '.candidate.source.commit | select(type == "string") | select(test("^[0-9a-f]{40}$"))' "$receipt")" || fail "candidate receipt source commit is invalid"
test -n "$source_commit" || fail "candidate receipt source commit is empty"
staging_run_id="$(jq -er '.candidate.artifact.stagingRunId | select(type == "number") | select(. > 0 and floor == .)' "$receipt")" || fail "candidate receipt staging run id is invalid"
staging_run_attempt="$(jq -er '.candidate.artifact.stagingRunAttempt | select(type == "number") | select(. > 0 and floor == .)' "$receipt")" || fail "candidate receipt staging run attempt is invalid"
provenance_run_id="$(jq -er '.provenance.runId | select(type == "number") | select(. > 0 and floor == .)' "$receipt")" || fail "candidate receipt provenance run id is invalid"
provenance_run_attempt="$(jq -er '.provenance.runAttempt | select(type == "number") | select(. > 0 and floor == .)' "$receipt")" || fail "candidate receipt provenance run attempt is invalid"
test "$staging_run_id" = "$provenance_run_id" || fail "candidate receipt run identity does not match provenance"
test "$staging_run_attempt" = "$provenance_run_attempt" || fail "candidate receipt attempt does not match provenance"
test "$(jq -er '.provenance.workflowPath' "$receipt")" = .github/workflows/runtime-candidate.yml || fail "candidate receipt workflow is not canonical"

legacy_branch="codexy/runtime-activation-v${version}"
branch="${legacy_branch}-staging-${staging_run_id}-${staging_run_attempt}"
git check-ref-format --branch "$branch" >/dev/null 2>&1 || fail "derived activation branch is invalid"

if open_prs="$(gh pr list --repo "$repository" --base main --state open --limit 101 --json number,headRefName)"; then
	:
else
	gh_status=$?
	printf '%s\n' "could not read open activation pull requests" >&2
	exit "$gh_status"
fi
printf '%s\n' "$open_prs" | jq -e 'type == "array" and all(.[]; (.number | type) == "number" and (.headRefName | type) == "string")' >/dev/null || fail "open activation pull-request state is invalid"
inventory_count="$(printf '%s\n' "$open_prs" | jq -er 'length')"
test "$inventory_count" -lt 101 || fail "open activation pull-request inventory is saturated"
matching="$(printf '%s\n' "$open_prs" | jq -er --arg branch "$branch" '[.[] | select(.headRefName == $branch)] | length')"
test "$matching" -le 1 || fail "duplicate activation pull requests: branch=$branch count=$matching"
competing="$(printf '%s\n' "$open_prs" | jq -er --arg legacy "$legacy_branch" --arg generation_prefix "$legacy_branch-staging-" --arg branch "$branch" '[.[] | select(.headRefName == $legacy or (.headRefName | startswith($generation_prefix))) | select(.headRefName != $branch) | .headRefName] | unique | join(",")')"
test -z "$competing" || fail "competing runtime activation pull request: $competing"

printf '%s\n' "$branch"
