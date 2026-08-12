# Merge And Main Sync

## Merge Rules

MUST NOT merge a PR until every review surface has been inspected and resolved.
MUST treat requested changes, actionable suggestions, unresolved review threads, stale concerns after new
commits, and PR comments that identify defects as blockers until addressed or
covered by an accepted no-change rationale.

Known process deviations are recorded here and MUST NOT be repaired by rewriting
protected `main` history:

- PR #18 was squash merged as `docs(license): correct copyright owner (#)`
  because the merge command did not carry the numeric PR identifier into the
  subject.
- PR #200 used PR title and squash subject
  `Require separate issues for dogfooding defects`.
- PR #201 used a non-Conventional PR title and squash subject.
- PR #202 used PR title and squash subject
  `Require descriptive child thread titles`.
- PR #203 used PR title `Refactor oversized Codexy skill instructions` and
  squash subject `Refactor oversized Codexy skill instructions (#203)`.

To prevent repeats, every merge MUST derive the PR number and title from an
explicit `gh pr view <number>` call and MUST validate the PR title, explicit
squash subject, and full merge message before merge.

Before merging, MUST inspect latest PR state, checks, reviews, comments, and review
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
`CHANGES_REQUESTED`, no latest maintainer or GitHub app review requests changes,
required checks have passed or been accepted as non-required, actionable PR
comments are addressed, and fixed or accepted review threads are resolved.
Every unresolved actionable review thread remains merge-blocking until the
current head proves the fix and the thread is resolved. Every non-outdated
thread MUST be resolved before merge or have a documented accepted no-change
rationale.

Default merge continuation is not permission to use `--admin`, merge stale or
unreviewed heads, ignore child-owned feedback, or leave actionable threads open.
MUST NOT skip PR-body preservation or merge before rerunning verification after
review responses.

## Squash Merge Body Preservation

When merge gates pass, merge through GitHub with squash merge and branch
deletion. The squash merge commit body/description MUST be the PR body exactly
as merged. Prefer a current `headRefOid` match. Capture the PR title, body,
head, authorization record, and merge message in one fresh remote read. The
installed generic admission hooks handle their matching merge-message lifecycle
gate; skill-authored commands MUST NOT derive an executable from repository
paths, cache paths, or `PLUGIN_ROOT`. The merge subject MUST be derived from the captured PR title, and the
merge body MUST be the captured remote body; arbitrary local body files are not
merge input.

`--auto` only waits for configured GitHub requirements, and `--admin` bypasses
requirements. MUST NOT use `--admin` to skip required checks or review-thread
cleanup.

## Post-Merge Main Sync

After merge, MUST update the configured default-branch worktree from the remote
and verify the current merge commit subject/body against the remote values
captured before merge. Transient merge evidence MUST be kept outside the
repository and removed whether the merge succeeds or fails.
If GitHub did not delete the remote topic branch, MUST delete it only after
confirming the PR was merged and no dependent work needs the branch:

```sh
git push origin --delete <branch>
```
