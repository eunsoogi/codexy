#!/bin/sh
set -eu

fail() {
  printf '%s\n' "runtime candidate source admission failed: $1" >&2
  exit 1
}

source_commit=${SOURCE_COMMIT:?SOURCE_COMMIT is required}
case "$source_commit" in *[!0-9a-f]*|'') fail "source SHA must be 40 lowercase hexadecimal characters" ;; esac
test "${#source_commit}" -eq 40 || fail "source SHA must be 40 lowercase hexadecimal characters"

pull_request=${EXACT_PR_NUMBER:-}
if test -z "$pull_request"; then
  git merge-base --is-ancestor "$source_commit" origin/main || fail "source SHA is not an ancestor of protected main"
  mode=protected-main
else
  case "$pull_request" in 0|0*|*[!0-9]*) fail "exact PR number must be a positive decimal integer" ;; esac
  repository=${GITHUB_REPOSITORY:?GITHUB_REPOSITORY is required}
  test -n "${GH_TOKEN:-}" || fail "GitHub API token is required for exact PR admission"
  if ! response=$(gh api --method GET --header 'Accept: application/vnd.github+json' "repos/$repository/pulls/$pull_request"); then
    fail "GitHub API could not fetch the exact pull request"
  fi
  printf '%s\n' "$response" | jq -e \
    --arg repository "$repository" --arg source "$source_commit" --argjson expected "$pull_request" '
      has("number") and .number == $expected and
      .state == "open" and has("merged_at") and .merged_at == null and
      .base.ref == "main" and .base.repo.full_name == $repository and
      .head.repo.full_name == $repository and .head.sha == $source and
      (.head.sha | test("^[0-9a-f]{40}$"))
    ' >/dev/null || fail "PR is not an open unmerged same-repository main PR at the exact source SHA"
  mode=exact-pr-head
fi

if test -n "${GITHUB_OUTPUT:-}"; then
  printf 'mode=%s\nsource_sha=%s\npull_request=%s\n' "$mode" "$source_commit" "$pull_request" >>"$GITHUB_OUTPUT"
fi
printf 'runtime candidate source admission: mode=%s source=%s pull_request=%s\n' "$mode" "$source_commit" "${pull_request:-none}"
