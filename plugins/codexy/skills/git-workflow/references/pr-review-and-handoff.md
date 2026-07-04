# PR Review And Handoff

## Completion-Handoff PR State

Before a PR-readiness handoff or final answer claims completion, validate that
the claim does not stop at an open PR when the requested outcome includes
completion or the default Codexy merge flow. MUST capture current PR state first:

```sh
pr=<pr>
owner=<owner>
repo=<repo>
gh pr view "$pr" --json number,state,isDraft,mergeStateStatus,reviewDecision,baseRefName,body,headRefName,headRefOid,url,labels,closingIssuesReferences,comments,reviews,latestReviews > pr-state.base.json
default_branch="$(gh repo view "$owner/$repo" --json defaultBranchRef --jq '.defaultBranchRef.name')"
closing_issue="$(
  jq -r '.body // ""
    | split("\n")
    | map(select((. | gsub("[[:space:]]"; "")) != ""))
    | last // ""
    | capture("^(Fixes|Closes|Resolves) #(?<number>[0-9]+)$").number? // empty' \
    pr-state.base.json
)"
if [ -n "$closing_issue" ] &&
  [ "$(jq -r '.baseRefName // ""' pr-state.base.json)" != "$default_branch" ]; then
  gh issue view "$closing_issue" --repo "$owner/$repo" --json number,url,labels \
    > pr-state.linkedIssue.json
  jq '{nodes:[{number,url,labels:{nodes:(.labels | map({name}))}}]}' \
    pr-state.linkedIssue.json > pr-state.linkedIssueReferences.json
else
  jq -n '{nodes:[]}' > pr-state.linkedIssueReferences.json
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
}' > pr-state.reviewThreads.pages.json
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
}' > pr-state.comments.pages.json
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
}' > pr-state.reviews.pages.json
jq '[.[].data.repository.pullRequest.reviewThreads.nodes[]] as $nodes
  | {nodes: $nodes, pageInfo: {hasNextPage: false, endCursor: null}}' \
  pr-state.reviewThreads.pages.json > pr-state.reviewThreads.json
jq '[.[].data.repository.pullRequest.comments.nodes[]]' \
  pr-state.comments.pages.json > pr-state.comments.json
jq '[.[].data.repository.pullRequest.reviews.nodes[]]' \
  pr-state.reviews.pages.json > pr-state.reviews.json
jq '.[0].data.repository | {repositoryLabels: .labels, defaultBranchRef} + (.pullRequest | {labels, closingIssuesReferences})' \
  pr-state.reviewThreads.pages.json > pr-state.labels.json
jq --slurpfile reviewThreads pr-state.reviewThreads.json \
  --slurpfile labels pr-state.labels.json \
  --slurpfile linkedIssueReferences pr-state.linkedIssueReferences.json \
  --slurpfile comments pr-state.comments.json \
  --slurpfile reviews pr-state.reviews.json \
  '. + $labels[0] + {linkedIssueReferences: $linkedIssueReferences[0], reviewThreads: $reviewThreads[0], comments: $comments[0], reviews: $reviews[0]}' \
  pr-state.base.json > pr-state.json
rm -f pr-state.base.json pr-state.reviewThreads.pages.json \
  pr-state.reviewThreads.json pr-state.comments.pages.json \
  pr-state.comments.json pr-state.reviews.pages.json \
  pr-state.reviews.json pr-state.labels.json \
  pr-state.linkedIssue.json pr-state.linkedIssueReferences.json
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
MUST include PR `headRefName`, PR `labels`, and `closingIssuesReferences` with
issue labels for default-branch PRs. For non-default-base stacked PRs where
GitHub ignores closing keywords, the PR state file MUST include
`linkedIssueReferences` with issue labels instead. When repository labels exist,
the PR state file MUST also include the repository label taxonomy as
`repositoryLabels`; an unlabeled PR is not ready merely because handoff prose
says no labels apply.

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

## Codex Review Gate

Codex connector review is a real merge gate when expected for the repository or
when the maintainer asks for it. MUST inspect Codex review state on the latest head:

```sh
gh pr view <pr> --json number,headRefOid,reviews,latestReviews,comments,reviewDecision,statusCheckRollup
gh api repos/<owner>/<repo>/pulls/<pr>/comments --paginate
gh api repos/<owner>/<repo>/issues/<pr>/comments --paginate
```

MUST identify Codex connector output by `performed_via_github_app.slug ==
"chatgpt-codex-connector"`, `user.login ==
"chatgpt-codex-connector[bot]"`, compact PR author text that appears as
`chatgpt-codex-connector`, or the GitHub App avatar/icon URL for that app.

If expected automatic review does not appear after a reasonable wait, request
it:

```sh
gh pr comment <pr> --body "@codex review"
```

Immediately before posting `@codex review`, the parent/orchestrator MUST re-read
the latest PR comments and reviews for the current head. The parent/orchestrator
MUST NOT post `@codex review` when that fresh state already contains a Codex
review request or current-head Codex review output. This duplicate guard is not
limited to requests with `eyes`; a just-posted child request is already enough
that the parent/orchestrator MUST wait or poll instead. Because plain PR
comments are not reliably commit-bound in captured PR state, the
parent/orchestrator MUST treat any existing `@codex review` request comment as
active unless stronger current-head evidence proves otherwise. Historical
evidence for this guard includes PR #255
comments
`https://github.com/eunsoogi/codexy/pull/255#issuecomment-4880788420` and
`https://github.com/eunsoogi/codexy/pull/255#issuecomment-4880788656`, where a
child and parent posted duplicate requests seven seconds apart without a fresh
parent read of the latest comments.

An `eyes` reaction on the request means Codex noticed it; it is not approval.
Eyes-only evidence on a current-head review request is not merge-ready. Actual
review text, inline review output, a recognized no-suggestion body, or a
maintainer override is required.

Codex review completion signals include inline review comments or suggestions,
top-level connector review results, or connector-authored no-major-issue text
such as `Didn't find any major issues`. Setup comments such as "create an
environment for this repo" are connector responses but not review completion.

If new commits are pushed after Codex review, request or wait for fresh review
on the new head only after the immediate pre-post comment/review re-read proves
that no current-head Codex review request or output exists.

## Child-Owned Review Feedback

The parent handoff MUST include PR number, latest head SHA, relevant comments
or review thread URLs, allowed files, expected return evidence, and stop
condition. For non-trivial lanes it MUST require goal tool usage,
todo/plan tool usage, multi-agent usage or concrete not-useful rationale,
unavailable-tool fallbacks, current-diff sentinel review findings, codegraph
evidence, and LSP status.

After the owning child pushes a review-response commit, the parent MUST inspect
unresolved review threads after child fixes and fresh current-head review, then
MUST verify that the current head addresses each completed review thread before
resolving it in GitHub.

Fixed or accepted review threads MUST be resolved in GitHub before the PR is
merged. The parent MUST NOT resolve a thread merely because a child said it was
fixed, a commit was pushed, or a fresh review was requested.
