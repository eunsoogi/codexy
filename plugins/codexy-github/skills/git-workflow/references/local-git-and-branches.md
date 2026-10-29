# Local Git And Branches

## Worktree Setup

When issue-sized implementation or an explicitly selected repository process
requires local branch/worktree setup, MUST discover the configured default
branch and its protection, fetch its required starting ref, and verify the
requested task branch is not already owned locally, remotely, or by another PR.
Create the task branch only after an issue or explicit issue-sized scope exists,
and keep it in an isolated worktree. MUST NOT implement directly on the default
branch. Ordinary authorized remote metadata operations do not require this local
setup merely because the plugin is installed.

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

## Accepted Child Cleanup

The parent Orchestrator MUST perform this sequence only for accepted completed
work, including when reminded by the user. Forwarding a request alone does not
establish progress. This flow preserves the #289 open-PR guard and #330
dirty-worktree protection; it MUST NOT replace native goal lifecycle or
terminal-handoff requirements.

1. MUST bind the task/host, issue, linked PRs, owned local branch (or none),
   exact branch head, and managed worktree identity from `list_artifacts` (or no
   worktree) to the current owner. MUST inspect current task/goal, GitHub,
   `git status --short --branch`, untracked and ignored files, and
   `git worktree list --porcelain`. MUST NOT infer ownership from a name alone.
2. MUST verify native goal completion, parent acceptance, and no pending PR, CI,
   review, fixup, follow-up, or delivery work. Idle, blocked, a terminal report,
   zero review threads, or a clean checkout alone MUST NOT qualify. An open
   linked PR or outstanding merge readback MUST retain the task, worktree, and
   branch. Explicit disposal MUST resolve or abandon that work first; it MUST
   NOT bypass an open PR by deleting its local resources.
3. For merged work, MUST read back each PR's exact final head, merge commit, and
   intended target from GitHub; fetch that target and verify the merge commit is
   on it. MUST reconcile the local branch head and changes with that final PR
   head, including any later local commits or uncommitted work. Squash merges
   need the final PR diff and target merge-commit diff/content: ancestry of the
   issue tip alone is insufficient. For unmerged disposal, MUST record the
   explicit parent decision, its scope, and recoverable state; disposal MUST NOT
   be represented as integration.
4. MUST preserve staged, unstaged, untracked, and required ignored data and
   confirm no active owner or pending work still needs the checkout. Unexpected
   dirty data MUST stop cleanup until its preservation/disposition is resolved.
   For a Codex-managed worktree, MUST preserve needed ignored files separately
   before calling `archive_worktree` with the exact attached identity. MUST read
   back successful archival and its recoverable snapshot via `list_artifacts`,
   plus checkout removal via `git worktree list --porcelain`. Failure, ambiguous
   results, shared/pinned/primary restrictions, or unavailable snapshot proof
   MUST stop cleanup; MUST NOT substitute shell worktree removal. MUST use the
   task whose attachment the tool can archive; an inaccessible child attachment
   MUST remain preserved for the owning task or host action.
5. Only after that proof, MUST archive the accepted completed task using
   `set_thread_archived(archived=true)` and confirm its archived identity using
   `list_archived_threads`. MUST NOT infer task archival from checkout removal.
   A failed task archive MUST retain its actual result and stop further cleanup.
   With neither an owned worktree nor branch, only task archival is needed; MUST
   NOT invent a resource to remove. An unmanaged checkout MUST remain preserved
   until its separately authorized preservation/removal path is resolved.
6. MUST recheck branch ownership, exact head, and all worktree checkouts before
   branch removal. Only the owned, unshared local issue branch with verified
   integration or explicit disposal and preserved recoverable state qualifies.
   MUST run ordinary `git branch -d -- <owned-local-issue-branch>` from a
   retained checkout on the intended target; MUST NOT force-delete, reset the
   branch tip, or change safety settings to bypass refusal. A squash-merged or
   disposed tip may fail Git's merged check despite the recorded evidence; MUST
   retain it and report the refusal for parent disposition. MUST NOT delete
   default/shared branches or any remote PR branch through this flow.
7. MUST read back local branch absence with
   `git show-ref --verify refs/heads/<owned-local-issue-branch>` and reconcile
   `git worktree list --porcelain`. MUST record task/issue/PR/branch/worktree
   identities, final PR head, merge SHA or disposal decision, preserved-state
   location, both archive results, and branch-removal result (or not
   applicable). At every step, uncertain ownership, outstanding work,
   dirty/ignored data, integration/disposal, archival, or deletion evidence MUST
   preserve the remaining resource and report the exact gap and next action.

Verification of this guidance MUST distinguish document/semantic review from
authentic host archive or Git removal behavior. Any live behavior check MUST use
explicitly authorized disposable state; MUST NOT experiment on live tasks or
valuable branches.
