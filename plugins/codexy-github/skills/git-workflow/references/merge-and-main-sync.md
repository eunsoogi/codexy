# Merge And Main Sync

## Pre-Merge Readback

MUST fresh-read the live PR, exact base/head, checks, reviews, comments, labels,
issue linkage, and review threads. Requested changes, actionable feedback,
unresolved actionable threads, stale proof, wrong targets, or missing
authorization block merge. MUST NOT use `--admin` to bypass a gate.

## Canonical Squash Mutation

Direct or nested `mcp__codex_apps__github_merge_pull_request` and auto-merge
connector mutations remain `UNAVAILABLE`.
After every independent gate and exact authorization passes, invoke only the
installed host-resolved resource:

```text
skills/git-workflow/scripts/codexy-authorized-squash-merge.sh
```

For the live PR, supply exact `--expected-pr`, `--expected-issue`, `--repo`,
`--match-head-commit`, `--subject`, `--body-file`, and `--merge-message-file`
values derived from one fresh authenticated PR/authorization capture. The
wrapper validates the target and merge-message payload, delegates to the
canonical hooked wrapper, performs a squash merge, requests branch deletion,
and MUST return zero before post-merge proof begins.

The squash subject MUST derive from the captured remote PR title. The squash
body MUST preserve the captured remote PR body exactly. Arbitrary local body or
authorization files are not authority.

## Post-Merge Proof

Using the live pre-merge PR number, head branch, base branch, and returned merge
SHA, perform this read-only connector sequence:

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

If the branch remains and no authenticated branch-delete connector mutation is
available, return `BLOCKED_MISSING_BRANCH_DELETE_SURFACE`; MUST NOT substitute
`gh` or another mutation. Any failed readback blocks post-merge completion.

Finally synchronize the configured default-branch worktree by fast-forward and
verify the merge commit again. Keep transient evidence outside the repository.
