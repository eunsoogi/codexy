#!/bin/sh
set -eu

fail() {
	printf '%s\n' "$1" >&2
	exit 1
}

validate_open_prs() {
	printf '%s\n' "$1" | jq -e '
		type == "array" and all(.[];
			(.number | type) == "number" and
			(.headRefName | type) == "string" and
				(.headRefOid | type) == "string" and
				(.baseRefName | type) == "string" and
				(.baseRefOid | type) == "string" and
			(.isCrossRepository | type) == "boolean" and
			(.headRepository | type) == "object" and
			(.headRepository.nameWithOwner | type) == "string" and
			(.headRepositoryOwner | type) == "object" and
			(.headRepositoryOwner.login | type) == "string"
		)' >/dev/null || fail "open activation pull-request state is invalid"
}

open_pr_snapshot() {
	branch=$1
	if open_prs="$(gh pr list --repo "$repository" --head "$branch" --state open --limit 101 --json number,headRefName,headRefOid,baseRefName,baseRefOid,isCrossRepository,headRepository,headRepositoryOwner)"; then
		:
	else
		gh_status=$?
		printf '%s\n' "could not read open activation pull requests" >&2
		exit "$gh_status"
	fi
	validate_open_prs "$open_prs"
	inventory_count="$(printf '%s\n' "$open_prs" | jq -er 'length')"
	test "$inventory_count" -lt 101 || fail "open activation pull-request inventory is saturated"
	wrong_base="$(printf '%s\n' "$open_prs" | jq -er --arg repository "$repository" --arg repository_owner "$repository_owner" --arg branch "$branch" '[.[] | select(.isCrossRepository == false and .headRepository.nameWithOwner == $repository and .headRepositoryOwner.login == $repository_owner and .headRefName == $branch and .baseRefName != "main") | .baseRefName] | unique | join(",")')"
	test -z "$wrong_base" || fail "activation pull request targets non-main base: $wrong_base"
	same_repo="$(printf '%s\n' "$open_prs" | jq -er --arg repository "$repository" --arg repository_owner "$repository_owner" --arg branch "$branch" '[.[] | select(.isCrossRepository == false and .headRepository.nameWithOwner == $repository and .headRepositoryOwner.login == $repository_owner and .headRefName == $branch and .baseRefName == "main")]')"
	count="$(printf '%s\n' "$same_repo" | jq -er 'length')"
	test "$count" -le 1 || fail "duplicate activation pull requests: branch=$branch count=$count"
	if test "$count" = 1; then
		pr_number="$(printf '%s\n' "$same_repo" | jq -er '.[0].number')"
		head_oid="$(printf '%s\n' "$same_repo" | jq -er '.[0].headRefOid')"
		base_oid="$(printf '%s\n' "$same_repo" | jq -er '.[0].baseRefOid')"
		printf '%s\t%s\t%s\t%s\n' "$count" "$pr_number" "$head_oid" "$base_oid"
	else
		printf '%s\tmissing\tmissing\tmissing\n' "$count"
	fi
}

if test "${1:-}" = --open-pr; then
	repository=${2:?repository is required}
	branch=${3:?activation branch is required}
	repository_owner=${repository%%/*}
	case "$repository" in
	*/?*) ;;
	*) fail "repository must be owner/name" ;;
	esac
	git check-ref-format --branch "$branch" >/dev/null 2>&1 || fail "activation branch is invalid"
	open_pr_snapshot "$branch"
	exit 0
fi

version=${1:?bootstrap version is required}
receipt=${2:?candidate receipt is required}
repository=${GITHUB_REPOSITORY:?repository is required}
repository_owner=${repository%%/*}
case "$repository" in
*/?*) ;;
*) fail "repository must be owner/name" ;;
esac

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

if open_prs="$(gh pr list --repo "$repository" --base main --state open --limit 101 --json number,headRefName,headRefOid,baseRefName,baseRefOid,isCrossRepository,headRepository,headRepositoryOwner)"; then
	:
else
	gh_status=$?
	printf '%s\n' "could not read open activation pull requests" >&2
	exit "$gh_status"
fi
validate_open_prs "$open_prs"
inventory_count="$(printf '%s\n' "$open_prs" | jq -er 'length')"
test "$inventory_count" -lt 101 || fail "open activation pull-request inventory is saturated"
matching="$(printf '%s\n' "$open_prs" | jq -er --arg repository "$repository" --arg repository_owner "$repository_owner" --arg branch "$branch" '[.[] | select(.isCrossRepository == false and .headRepository.nameWithOwner == $repository and .headRepositoryOwner.login == $repository_owner and .headRefName == $branch and .baseRefName == "main")] | length')"
test "$matching" -le 1 || fail "duplicate activation pull requests: branch=$branch count=$matching"
competing="$(printf '%s\n' "$open_prs" | jq -er --arg repository "$repository" --arg repository_owner "$repository_owner" --arg legacy "$legacy_branch" --arg generation_prefix "$legacy_branch-staging-" --arg branch "$branch" '[.[] | select(.isCrossRepository == false and .headRepository.nameWithOwner == $repository and .headRepositoryOwner.login == $repository_owner and .baseRefName == "main" and (.headRefName == $legacy or (.headRefName | startswith($generation_prefix)))) | select(.headRefName != $branch) | .headRefName] | unique | join(",")')"
test -z "$competing" || fail "competing runtime activation pull request: $competing"
open_pr_snapshot "$branch" >/dev/null

printf '%s\n' "$branch"
