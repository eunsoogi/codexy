# Merge And Main Sync

## Merge Rules

MUST NOT merge a PR until every review surface has been inspected and resolved.
Codex connector reviews are merge-blocking reviews. Treat requested changes,
actionable suggestions, unresolved review threads, stale concerns after new
commits, and PR comments that identify defects as blockers until addressed or
covered by an accepted no-change rationale.

PR #18 was squash merged as `docs(license): correct copyright owner (#)` because
the merge command did not carry the numeric PR identifier into the subject. Do
not rewrite protected `main` history to repair that old commit. Prevent repeats
by deriving the PR number from an explicit `gh pr view <number>` call before
every merge.

Before merging, inspect latest PR state, checks, reviews, comments, and review
threads:

```sh
gh pr view <pr> --json number,title,state,headRefName,headRefOid,baseRefName,mergeStateStatus,statusCheckRollup,reviewDecision,latestReviews,reviews,comments,labels,closingIssuesReferences
gh pr view <pr> --comments
gh api graphql -f owner=<owner> -f name=<repo> -F number=<pr-number> -f query='
query($owner:String!, $name:String!, $number:Int!) {
  repository(owner:$owner, name:$name) {
    pullRequest(number:$number) {
      reviewThreads(first:100) {
        nodes { id isResolved isOutdated path comments(first:20) { nodes { author { login } body url createdAt } } }
      }
    }
  }
}'
```

The review gate is satisfied only when `reviewDecision` is not
`CHANGES_REQUESTED`, no latest maintainer/GitHub app/Codex connector review
requests changes, expected Codex review completed on latest `headRefOid`,
required checks have passed or been accepted as non-required, actionable PR
comments are addressed, and fixed or accepted review threads are resolved.
Every unresolved actionable review thread remains merge-blocking until the
current head proves the fix and the thread is resolved. Every non-outdated
thread MUST be resolved before merge or have a documented accepted no-change
rationale.

Default merge continuation is not permission to use `--admin`, merge stale or
unreviewed heads, ignore child-owned feedback, leave actionable threads open,
skip PR-body preservation, or merge before rerunning verification after review
responses.

## Squash Merge Body Preservation

When merge gates pass, merge through GitHub with squash merge and branch
deletion. The squash merge commit body/description MUST be the PR body exactly
as merged. Prefer `--match-head-commit <headRefOid>`.

```bash
set -euo pipefail

pr_number=<explicit-pr-number>
issue_number=<linked-issue-number>
repo=eunsoogi/codexy
merge_subject="<conventional subject> (#${pr_number})"
pr_json_file=$(mktemp)
pr_body_file=$(mktemp)
merge_message_file=$(mktemp)
git_common_dir=$(cd "$(git rev-parse --git-common-dir)" && pwd -P)
expected_body_file="${git_common_dir}/codexy/merge-bodies/pr-${pr_number}.body"
trap 'rm -f "$pr_json_file" "$pr_body_file" "$merge_message_file"' EXIT

if ! head_oid=$(gh pr view "$pr_number" --repo "$repo" --json headRefOid --jq .headRefOid); then
  printf '%s\n' "failed to capture PR headRefOid" >&2
  exit 1
fi
if ! gh pr view "$pr_number" --repo "$repo" --json body > "$pr_json_file"; then
  printf '%s\n' "failed to capture PR body JSON" >&2
  exit 1
fi
if ! ruby -rjson -e 'body = JSON.parse(File.binread(ARGV.fetch(0))).fetch("body"); abort("captured PR body is empty") if body.nil? || body.empty?; File.binwrite(ARGV.fetch(1), body)' "$pr_json_file" "$pr_body_file"; then
  printf '%s\n' "failed to extract PR body" >&2
  exit 1
fi
printf '%s\n\n' "$merge_subject" > "$merge_message_file"
cat "$pr_body_file" >> "$merge_message_file"

merge_validation_args=(--check-merge-message --expected-pr "$pr_number")
if [ -n "${issue_number:-}" ]; then
  merge_validation_args+=(--expected-issue "$issue_number")
fi
if ! scripts/validate-plugin-config "${merge_validation_args[@]}" --merge-message-file "$merge_message_file"; then
  printf '%s\n' "merge message validation failed" >&2
  exit 1
fi

printf '%s\n' "Inspect the captured PR body before merge: $pr_body_file"
printf '%s\n' "It MUST NOT contain secrets, credentials, private logs, throwaway notes, or local-only scratch paths unless intentional evidence references."
if [ -t 1 ] && command -v less >/dev/null 2>&1; then
  if ! less "$pr_body_file"; then
    printf '%s\n' "failed to display captured PR body with less" >&2
    exit 1
  fi
elif ! cat "$pr_body_file"; then
  printf '%s\n' "failed to display captured PR body" >&2
  exit 1
fi
printf '%s' "Type APPROVE_PR_BODY_FOR_MAIN to continue: "
IFS= read -r pr_body_approval
if [ "$pr_body_approval" != "APPROVE_PR_BODY_FOR_MAIN" ]; then
  printf '%s\n' "PR body approval token mismatch" >&2
  exit 1
fi

mkdir -p "$(dirname "$expected_body_file")"
if ! cp "$pr_body_file" "$expected_body_file"; then
  printf '%s\n' "failed to store expected merge body" >&2
  exit 1
fi
if ! test -s "$expected_body_file"; then
  printf '%s\n' "stored expected merge body is empty" >&2
  exit 1
fi

if ! gh pr merge "$pr_number" \
  --repo "$repo" \
  --squash \
  --delete-branch \
  --match-head-commit "$head_oid" \
  --subject "$merge_subject" \
  --body-file "$pr_body_file"; then
  printf '%s\n' "GitHub squash merge failed" >&2
  exit 1
fi
```

`gh pr merge` has no flag that means "Codex review passed." `--auto` only waits
for configured GitHub requirements, and `--admin` bypasses requirements. MUST NOT
use `--admin` to skip Codex review, required checks, or review-thread cleanup.

## Post-Merge Main Sync

After merge, update the main worktree and verify the merge body:

```bash
set -euo pipefail

pr_number=<explicit-pr-number>
git_common_dir=$(cd "$(git rev-parse --git-common-dir)" && pwd -P)
expected_body_file="${git_common_dir}/codexy/merge-bodies/pr-${pr_number}.body"
commit_message_file=$(mktemp)
trap 'rm -f "$commit_message_file"' EXIT
if ! test -f "$expected_body_file"; then
  printf '%s\n' "missing expected merge body file" >&2
  exit 1
fi

if ! git pull --ff-only origin main; then
  printf '%s\n' "failed to fast-forward main" >&2
  exit 1
fi
git log -1 --pretty=%s
if ! git cat-file commit HEAD | ruby -e 'input = STDIN.read; print input.split("\n\n", 2).fetch(1)' > "$commit_message_file"; then
  printf '%s\n' "failed to capture merge commit body" >&2
  exit 1
fi
ruby - "$expected_body_file" "$commit_message_file" <<'RUBY'
expected = File.binread(ARGV.fetch(0))
raw_message = File.binread(ARGV.fetch(1))
actual = raw_message.include?("\n\n") ? raw_message.split("\n\n", 2).fetch(1) : ""
strip_one = ->(value) { value.end_with?("\n") ? value[0...-1] : value }
abort("squash merge commit body does not match the captured PR body") unless strip_one.call(actual) == strip_one.call(expected)
RUBY

expected_issue_number=$(ruby - "$expected_body_file" <<'RUBY'
body = File.binread(ARGV.fetch(0))
match = body.lines.map(&:strip).reverse.find { |line| line.match?(/\AFixes #\d+\z/) }
print(match[/\d+/]) if match
RUBY
)
post_merge_validation_args=(--check-merge-message --expected-pr "$pr_number")
if [ -n "$expected_issue_number" ]; then
  post_merge_validation_args+=(--expected-issue "$expected_issue_number")
fi
if ! scripts/validate-plugin-config "${post_merge_validation_args[@]}" --merge-message-file "$commit_message_file"; then
  printf '%s\n' "post-merge message validation failed" >&2
  exit 1
fi
rm -f "$expected_body_file"
```

The refreshed `main` commit subject MUST end with `(#<merged-pr-number>)`, and
the refreshed `main` commit body MUST match the PR body captured before merge.
If GitHub did not delete the remote topic branch, delete it only after
confirming the PR was merged and no dependent work needs the branch:

```sh
git push origin --delete <branch>
```
