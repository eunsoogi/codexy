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

Immediately before mutation, MUST perform this sequence in order, regardless of
the invocation route:

1. Fresh-read the authenticated remote PR's repository, number, title, body,
   base, and head SHA. MUST use the actual returned PR number, not an issue
   number or a remembered example. Missing or invalid metadata MUST stop the
   merge; MUST NOT guess a title, number, or head.
2. Validate the captured PR title separately using the retained PR-title rules:
   `type(scope): description`, with no issue or PR reference. MUST NOT append a
   suffix to an invalid title or repair the captured title locally.
3. Build one final subject value: captured title + one ASCII space +
   `(#<captured PR number>)`. Validate that complete value against the retained
   Conventional Commit and exact PR-number rules. MUST reject a missing,
   different, or duplicate suffix even when the PR title itself is valid.
4. Send that exact validated subject through the normal merge route: connector
   `commit_title` or `gh pr merge --subject`. MUST preserve the captured body
   exactly as connector `commit_message` or CLI `--body`, and bind the request
   to the captured repository, PR number, and head using the route's head-match
   guard when available. MUST NOT reconstruct, shorten, or substitute the
   subject after validation. Arbitrary local body or authorization files are not
   authority.

If remote metadata changes or a head-match guard fails before mutation, MUST
discard the prepared request, fresh-read the PR, and repeat the sequence and any
affected authorization, checks, or review gates. MUST NOT reuse a validated
subject from the old capture or claim both captures are one. A fresh read does
not make title/body updates atomic with merge; an unavailable head-match guard
MUST remain an explicit limitation, not a server-side guarantee.

A hook returning no denial MUST NOT substitute for explicit validation of the
final value. Arbitrary Python subprocess construction and other unsupported
dynamic execution are not inspected, and a hook replay does not prove live host
dispatch or denial consumption.

### Worked subject checks

For a fresh PR #321 with title `ci(ios): add Duo validation profile` and head
`H`, the title check passes. The final subject check is a separate step:

| Candidate subject or changed metadata                                       | Required result                                                                                                                                                     |
| --------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `ci(ios): add Duo validation profile (#321)`                                | Accept; pass this exact string as `commit_title` or `--subject` for PR #321 at `H`.                                                                                 |
| `ci(ios): add Duo validation profile`                                       | Reject: missing suffix despite a valid title.                                                                                                                       |
| `ci(ios): add Duo validation profile (#320)`                                | Reject: wrong PR number.                                                                                                                                            |
| `ci(ios): add Duo validation profile (#321) (#321)`                         | Reject: duplicate suffix.                                                                                                                                           |
| Missing title/number/head, or title `add Duo validation profile`            | Stop: metadata or PR-title validation failed before subject construction.                                                                                           |
| Title becomes `ci(ios): refine Duo validation profile` or head becomes `H2` | Discard the old request; recapture and revalidate before sending. For the new title, send only the newly validated `ci(ios): refine Duo validation profile (#321)`. |

## Post-Merge Proof

After a squash merge receipt, MUST read the returned merge commit's subject and
compare it exactly with the subject validated and sent above. A missing commit
readback or any mismatch MUST stop the success claim; MUST report the expected
and observed subject and leave the merge outcome unverified. MUST NOT rewrite
history to repair a mismatch without separate authorization.

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
`BLOCKED_MISSING_BRANCH_DELETE_SURFACE`; do not claim post-merge completion. Any
failed readback blocks only the corresponding post-merge claim.

Finally synchronize the configured default-branch worktree by fast-forward and
verify the merge commit again. Keep transient evidence outside the repository.
