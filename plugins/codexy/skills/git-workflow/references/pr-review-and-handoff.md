# PR Review And Handoff

## Completion-Handoff PR State

Before a PR-readiness handoff or final answer claims completion, validate that
the claim does not stop at an open PR when the requested outcome includes
completion or the default Codexy merge flow. MUST capture current PR state first:

```sh
pr=<pr>
owner=<owner>
repo=<repo>
state_dir=$(mktemp -d)
trap 'rm -rf "$state_dir"' EXIT
gh pr view "$pr" --json number,state,isDraft,mergeStateStatus,reviewDecision,baseRefName,body,headRefName,headRefOid,url,labels,closingIssuesReferences,comments,reviews,latestReviews > "$state_dir/pr-state.base.json"
head_ref="$(jq -r '.headRefName' "$state_dir/pr-state.base.json")"
git fetch origin "$head_ref"
git status --short --branch > "$state_dir/worktreeStatus.txt"
git rev-parse HEAD > "$state_dir/localHeadOid.txt"
git rev-parse "origin/$head_ref" > "$state_dir/remoteHeadOid.txt"
default_branch="$(gh repo view "$owner/$repo" --json defaultBranchRef --jq '.defaultBranchRef.name')"
closing_issue="$(
  jq -r '.body // ""
    | split("\n")
    | map(select((. | gsub("[[:space:]]"; "")) != ""))
    | last // ""
    | capture("^(Fixes|Closes|Resolves) #(?<number>[0-9]+)$").number? // empty' \
    "$state_dir/pr-state.base.json"
)"
if [ -n "$closing_issue" ] &&
  [ "$(jq -r '.baseRefName // ""' "$state_dir/pr-state.base.json")" != "$default_branch" ]; then
  gh issue view "$closing_issue" --repo "$owner/$repo" --json number,url,labels \
    > "$state_dir/linkedIssue.json"
  jq '{nodes:[{number,url,labels:{nodes:(.labels | map({name}))}}]}' \
    "$state_dir/linkedIssue.json" > "$state_dir/linkedIssueReferences.json"
else
  jq -n '{nodes:[]}' > "$state_dir/linkedIssueReferences.json"
fi
gh api graphql --paginate --slurp \
  -f owner="$owner" -f name="$repo" -F number="$pr" -f query='
query($owner:String!, $name:String!, $number:Int!, $endCursor:String) {
  repository(owner:$owner, name:$name) {
    defaultBranchRef { name }
    labels(first:100) { nodes { name } }
    pullRequest(number:$number) {
      labels(first:50) { nodes { name } }
      closingIssuesReferences(first:20) { nodes { number labels(first:50) { nodes { name } } } }
      reviewThreads(first:100, after:$endCursor) {
        pageInfo { hasNextPage endCursor }
        nodes { id isResolved isOutdated path comments(first:20) { nodes { author { login } body url createdAt commit { oid } } } }
      }
    }
  }
}' > "$state_dir/reviewThreads.pages.json"
gh api graphql --paginate --slurp \
  -f owner="$owner" -f name="$repo" -F number="$pr" -f query='
query($owner:String!, $name:String!, $number:Int!, $endCursor:String) {
  repository(owner:$owner, name:$name) {
    pullRequest(number:$number) {
      comments(first:100, after:$endCursor) {
        pageInfo { hasNextPage endCursor }
        nodes {
          author { login }
          body
          url
          createdAt
          reactionGroups { content users { totalCount } }
        }
      }
    }
  }
}' > "$state_dir/comments.pages.json"
gh api graphql --paginate --slurp \
  -f owner="$owner" -f name="$repo" -F number="$pr" -f query='
query($owner:String!, $name:String!, $number:Int!, $endCursor:String) {
  repository(owner:$owner, name:$name) {
    pullRequest(number:$number) {
      reviews(first:100, after:$endCursor) {
        pageInfo { hasNextPage endCursor }
        nodes { author { login } body state url submittedAt commit { oid } }
      }
    }
  }
}' > "$state_dir/reviews.pages.json"
jq '[.[].data.repository.pullRequest.reviewThreads.nodes[]] as $nodes
  | {nodes: $nodes, pageInfo: {hasNextPage: false, endCursor: null}}' \
  "$state_dir/reviewThreads.pages.json" > "$state_dir/reviewThreads.json"
jq '[.[].data.repository.pullRequest.comments.nodes[]]' \
  "$state_dir/comments.pages.json" > "$state_dir/comments.json"
jq '[.[].data.repository.pullRequest.reviews.nodes[]]' \
  "$state_dir/reviews.pages.json" > "$state_dir/reviews.json"
jq '.[0].data.repository
  | {repositoryLabels: .labels, defaultBranchRef}
    + (.pullRequest | {
        labels,
        closingIssuesReferences,
      })' \
  "$state_dir/reviewThreads.pages.json" > "$state_dir/labels.json"
jq --slurpfile reviewThreads "$state_dir/reviewThreads.json" \
  --slurpfile labels "$state_dir/labels.json" \
  --slurpfile linkedIssueReferences "$state_dir/linkedIssueReferences.json" \
  --slurpfile comments "$state_dir/comments.json" \
  --slurpfile reviews "$state_dir/reviews.json" \
  --rawfile worktreeStatus "$state_dir/worktreeStatus.txt" \
  --rawfile localHeadOid "$state_dir/localHeadOid.txt" \
  --rawfile remoteHeadOid "$state_dir/remoteHeadOid.txt" \
  '. + $labels[0] + {linkedIssueReferences: $linkedIssueReferences[0], worktreeStatus: $worktreeStatus, localHeadOid: ($localHeadOid | gsub("\n$"; "")), remoteHeadOid: ($remoteHeadOid | gsub("\n$"; "")), reviewThreads: $reviewThreads[0], comments: $comments[0], reviews: $reviews[0]}' \
  "$state_dir/pr-state.base.json" > pr-state.json
scripts/validate-plugin-config --check-completion-handoff \
  --handoff-file <report> \
  --pr-state-file pr-state.json
```

For stacked PRs whose `baseRefName` is not the captured `defaultBranchRef.name`,
GitHub does not populate PR `closingIssuesReferences` from closing keywords. The
PR state file MUST still include comparable authoritative issue evidence before
readiness. It MUST keep the PR `body` final closing-keyword line and MUST add
`linkedIssueReferences.nodes[]` for that issue, including `number`, `url`, and
`labels.nodes[].name`, captured from the same GitHub repository's issue or
GraphQL API output.

For review-response handoffs, the PR state file MUST include GraphQL
`reviewThreads.nodes` with `id`, `isResolved`, `isOutdated`, `path`, and
comment URLs. For PR-readiness or merge-readiness handoffs, the PR state file
MUST include PR `headRefName`, PR `labels`, and
`closingIssuesReferences` with issue labels for default-branch PRs. For
non-default-base stacked PRs where GitHub ignores closing keywords, the PR state
file MUST include
`linkedIssueReferences` with issue labels instead. When repository labels exist,
the PR state file MUST also include the repository label taxonomy as
`repositoryLabels`; an unlabeled PR is not ready merely because handoff prose
says no labels apply.
For child handoffs that claim pushed or synced branch state, the PR state file
MUST include the local `git status --short --branch` output as `worktreeStatus`;
missing branch-status evidence blocks the handoff because stale local branches
MUST NOT be ruled out without local branch-status evidence.
For child handoffs that claim parent acceptance, merge evaluation, or PR
readiness, the PR state file MUST include captured local `HEAD` as
`localHeadOid` and the PR branch remote-tracking ref as `remoteHeadOid`.

Before PR readiness, the owning lane MUST run the hard PR title hook with the
exact GitHub PR title:

```sh
plugins/codexy/hooks/codexy-pr-title-check.sh --pr-title "$(gh pr view "$pr" --json title --jq .title)"
```

Before PR readiness, the owning lane MUST run the hard PR label hook against
captured PR state with `repositoryLabels`:

```sh
plugins/codexy/hooks/codexy-pr-label-check.sh --pr-state-file pr-state.json
```

Completion-handoff validation MUST run in the same readiness path. Linked issue
labels and repository label evidence MUST NOT be skipped after the label hook
passes:

```sh
scripts/validate-plugin-config --check-completion-handoff \
  --handoff-file <report> \
  --pr-state-file pr-state.json
```

## Child-Owned Review Feedback

The parent handoff MUST include PR number, latest head SHA, relevant comments
or review thread URLs, allowed files, expected return evidence, and stop
condition. For non-trivial lanes it MUST require goal tool usage,
todo/plan tool usage, multi-agent usage or concrete not-useful rationale,
unavailable-tool fallbacks, current-diff sentinel review findings, codegraph
evidence, and LSP status.

After the owning child pushes a review-response commit, the parent MUST inspect
unresolved review threads after child fixes and refreshed thread capture, then
MUST verify that the current head addresses each completed review thread before
resolving it in GitHub.

Fixed or accepted review threads MUST be resolved in GitHub before the PR is
claimed PR-ready, merge-ready, or merged. The parent MUST NOT resolve a thread
merely because a child said it was fixed, a commit was pushed, or a fresh review
was requested.
