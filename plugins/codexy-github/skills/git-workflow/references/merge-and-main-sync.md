# Merge And Main Sync

## Pre-Merge Readback

When a repository or maintainer selects this merge evidence contract, MUST
fresh-read the live PR, exact base/head, checks, reviews, comments, labels,
issue linkage, and review threads. Requested changes, actionable feedback,
unresolved actionable threads, stale proof, wrong targets, or missing
authorization block that selected merge process. MUST NOT use `--admin` to
bypass a gate. Otherwise use the normal host or connector route and GitHub's
server-side response without requiring this plugin-owned preparation.

## Authorized Squash Mutation

After every independent gate and exact authorization passes for the selected
contract, request the squash merge through the normal host or connector route.
Keep the live PR, repository, base, head, and merge-message values from one
fresh authenticated capture, and use the host/connector/GitHub response as the
mutation receipt. The plugin does not provide a canonical merge wrapper or a
replacement admission decision.

The squash subject MUST derive from the captured remote PR title. The squash
body MUST preserve the captured remote PR body exactly. Arbitrary local body or
authorization files are not authority.

## Post-Merge Proof

When post-merge proof is requested under the selected contract, use the live
pre-merge PR number, head branch, base branch, and returned merge SHA for this
read-only connector sequence:

1. `mcp__codex_apps__github_fetch_pr`: confirm merged state, unchanged base/head
   names, and merge SHA;
2. `mcp__codex_apps__github_search_branches`: search the exact head branch and
   confirm it is absent;
3. `mcp__codex_apps__github_search_branches`: search the exact protected base
   branch and capture its current head;
4. `mcp__codex_apps__github_compare_commits`: compare the merge SHA to that base
   and require `identical` or `ahead` with `behind_by` zero;
5. `mcp__codex_apps__github_fetch_commit`: confirm the merge commit's canonical
   URL and captured subject/body; and
6. `mcp__codex_apps__github_get_commit_combined_status`: require every necessary
   post-merge status on the current base head to succeed.

If that selected cleanup contract requires branch deletion and the branch
remains without an authenticated branch-delete surface, return
`BLOCKED_MISSING_BRANCH_DELETE_SURFACE`; do not claim post-merge completion.
Any failed readback blocks only the corresponding post-merge claim.

Finally synchronize the configured default-branch worktree by fast-forward and
verify the merge commit again. Keep transient evidence outside the repository.
