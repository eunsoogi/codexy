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

## Unfinished Work Preservation

For an explicitly authorized stop-and-preserve request, the issue owner MUST
inspect the current worktree and relevant diffs before changing its files or
branch. The inventory MUST identify the managed worktree, branch and exact head,
staged and unstaged changes, untracked files, required ignored data, remaining
obligations and proof, and a concrete resume location. Inspect ignored data only
as needed to establish what continuation requires; credentials, private logs,
and sensitive local configuration MUST NOT be copied into commits, pushed
branches, or Draft PRs. Preserve required sensitive data only in an already
authorized local or secure location, and report its availability without
exposing its contents.

The request's explicit scope governs each preservation action. The owner MUST
assess authorization to commit, push, and create or update a Draft PR
independently; authorization for one action MUST NOT imply another. When commit
is authorized, the owner MUST commit only owned, non-sensitive changes. When
push is authorized, the owner MUST push the issue branch. When Draft PR creation
or update is authorized, the owner MUST create or update a Draft PR that clearly
says the work is incomplete, lists remaining obligations, and gives a concrete
resume point. The owner MUST report checks that remain unrun or failed without
implying readiness. An unauthorized or failed action MUST be reported with its
exact state without misclassifying the authority or result of other actions;
retain the local branch and worktree when local preservation is sufficient, and
do not ask for unchanged confirmation.

Worktree archival MUST NOT be treated as preservation of required ignored data;
keep such data separately in an authorized location and verify its availability
before archival. Before task or worktree archival, verify the authorized
preserved location will remain accessible afterward; if that host behavior is
unknown, retain the affected task and checkout and report the boundary. A stop
or usage limit alone MUST NOT authorize remote publication, archival, or
deletion. Before archival, the parent MUST verify recoverable work, remaining
obligations, the resume location, and exactly one continuation owner. If
preservation fails or remains unknown, defer archival only for the affected
child and preserve its branch, worktree, and required ignored data. The
accepted-child cleanup below MUST NOT be applied to unfinished work.

## Accepted Child Cleanup

The parent Orchestrator MUST enter this sequence as its next action after
accepting a child's completed work and completing the required merge,
publication, follow-up, and delivery work. It MUST NOT wait for another
reminder. A reminder also resumes the sequence; receiving or forwarding one does
not establish progress. This flow preserves the #289 open-PR guard and
dirty-worktree protection documented in #330. It MUST NOT replace native goal
lifecycle or terminal-handoff requirements.

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
4. Before each destructive step, MUST verify that existing authority covers that
   exact action and child. Previously granted authority MUST be used without
   asking again. Acceptance or completion alone MUST NOT create missing
   destructive authority. If authority is missing, MUST retain the affected
   child or dependent action and record the exact gap; MUST continue other
   eligible children.
5. MUST preserve staged, unstaged, untracked, and required ignored data and
   confirm no active owner or pending work still needs the checkout. Unexpected
   dirty data MUST stop cleanup until its preservation/disposition is resolved.
   The parent MUST judge and perform necessary preservation; MUST NOT require
   worktree archival, substitute it for direct thread archival or branch
   removal, or delegate this cleanup to the child. MUST NOT remove a worktree
   with a shell command.
6. After those checks, the parent MUST archive the accepted completed task using
   `set_thread_archived(threadId=<child>, archived=true)` and confirm its exact
   archived identity using `list_archived_threads`. A failed task archive MUST
   retain its actual result and stop further cleanup for that child. With no
   owned local branch, branch deletion is not applicable; MUST NOT invent a
   resource to remove. Existing checkouts remain preserved.
7. MUST recheck branch ownership, exact head, and all worktree checkouts before
   branch removal. Only the owned, unshared local issue branch with verified
   integration or explicit disposal and preserved recoverable state qualifies.
   If the confirmed archived child's clean, owned checkout still has that branch
   checked out, the parent MAY detach it at its verified final commit while
   preserving its files and history, then recheck all worktree checkouts. MUST
   run ordinary `git branch -d -- <owned-local-issue-branch>` from a retained
   checkout on the intended target; MUST NOT force-delete, reset the branch tip,
   or change safety settings to bypass refusal. A squash-merged or disposed tip
   may fail Git's merged check despite the recorded evidence; MUST retain it and
   report the refusal for parent disposition. MUST NOT delete default/shared
   branches or any remote PR branch through this flow. A refusal defers branch
   removal for that child; it MUST NOT stop other eligible children.
8. MUST read back local branch absence with
   `git show-ref --verify refs/heads/<owned-local-issue-branch>` and reconcile
   `git worktree list --porcelain`. MUST record task/issue/PR/branch/worktree
   identities, final PR head, merge SHA or disposal decision, preserved-state
   location, thread archive and readback results, and branch-removal result (or
   not applicable). At every step, uncertain ownership, outstanding work,
   dirty/ignored data, integration/disposal, archival, or deletion evidence MUST
   preserve the affected resource and report the exact gap and next action.

For accepted children already represented in the current task state, MUST
evaluate and record cleanup per child. A specific gap in outstanding work,
preservation, ownership or shared use, authority, or host capability MUST defer
only its affected child or dependent action; MUST continue cleanup for other
eligible children. MUST retain each accepted child whose cleanup remains
pending, the completed steps, exact deferral, and next action in existing task
state across turns and compaction. On relevant progress or task resumption, MUST
re-read the actual state and resume eligible cleanup without a new reminder.
Before each destructive operation, MUST check whether its result is already
confirmed. MUST reuse confirmed thread-archive and branch-removal readbacks and
MUST NOT repeat those operations because of a duplicate or stale event. If a
prior result is ambiguous, MUST read back the actual thread or branch state
before choosing the next action. MUST NOT add a global child scan, workflow
database, or background cleanup service.

Verification of this guidance MUST distinguish document/semantic review from
authentic host archive or Git removal behavior. Any live behavior check MUST use
explicitly authorized disposable state; MUST NOT experiment on live tasks or
valuable branches.
