# Local Git And Branches

## Worktree Setup

MUST discover the configured default branch and its protection, fetch its
required starting ref, and verify the requested task branch is not already owned
locally, remotely, or by another PR. Create the task branch only after an issue
or explicit issue-sized scope exists, and keep it in an isolated worktree. MUST
NOT implement directly on the default branch.

Use the repository naming policy or the maintainer-requested branch name. Keep
the branch aligned to one issue and preserve unrelated user or agent work.

## Local Change Discipline

Before editing, staging, committing, pushing, or resolving conflicts, MUST
inspect `git status --short --branch` and the relevant diff. Stage only intended
paths. MUST NOT discard unrelated changes, commit local evidence or secrets, or
use `git push --force` or `git push --force-with-lease`.

Commit messages MUST use a descriptive Conventional Commit subject. If an
ordinary push is rejected, refresh the remote branch, inspect its changes, and
integrate safely with a new commit; do not rewrite the remote history.

For conflicts, inspect both sides and preserve their intended behavior. If the
domain choice is ambiguous, stop for maintainer direction. After resolution,
stage only resolved paths and rerun affected verification.
