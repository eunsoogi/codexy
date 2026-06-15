---
name: git-workflow
description: Codexy plugin GitHub issue, branch, worktree, push, pull request, verification, repository-settings, branch-protection, Codex review, and squash-merge workflow. Use before Git, issue, PR, label, review, protection, merge, or post-merge sync work in this repository.
---

# Git Workflow

Use this skill before Codexy Git, GitHub issue, branch, worktree, commit, push, pull request, review, repository-settings, branch-protection, merge, or post-merge sync work.

## Authority

`AGENTS.md` is the repository policy source. Direct user instructions and GitHub issue scope define the active task. This skill is the executable workflow. If this skill conflicts with `AGENTS.md`, follow `AGENTS.md`.

Use GitHub and `gh` for issue, pull request, review, check, label, branch-protection, repository-settings, and merge state when connector tools are not already handling that surface.

Use local `git` for local worktree inspection, checkout, worktree creation, diff, staging, committing, rebasing, pulling, and ordinary push.

## Start Work

1. Read `AGENTS.md` and this skill.
2. Create or confirm a GitHub issue before implementation. If the user provided an issue, treat that issue as the source of truth.
3. For non-trivial work, keep a short plan and update it as evidence changes.
4. Keep `main` as the protected integration branch. Do not implement directly on `main`.
5. Create a branch only after the issue or explicit issue-sized scope exists.
6. Use an isolated git worktree for the task branch.
7. Use the `codexy/` branch prefix unless the user requests another naming scheme.
8. Keep the branch scope aligned with the issue. Do not touch files outside the requested scope.

Issue titles should summarize the user-visible problem or needed work in plain prose. They must start with an uppercase letter and must not use Conventional Commit prefixes such as `feat(...)`.

Issue bodies should include:

- `## Problem`: the user-visible problem, requested workflow change, or defect.
- `## Scope`: the files, behavior, or workflow areas expected to change.
- `## Acceptance Criteria`: concrete conditions that make the issue done.
- `## Verification`: expected local checks or evidence.

When labels are available, inspect the repository's current label taxonomy
before creating or updating issues. Apply repository-appropriate labels for the
work type, status, priority, and ownership area only when those concepts exist
in that repository. Do not assume a universal label list across repositories.
If the current taxonomy is missing a minimal label needed for clear workflow
state, create or update the smallest repository-appropriate label set first,
then apply it.

## Worktrees And Branches

Create task worktrees from an up-to-date `main`:

```sh
git fetch origin main
git switch main
git pull --ff-only origin main
git worktree add -b codexy/<issue-or-scope> ../<repo>-worktrees/<issue-or-scope> main
```

If `origin/main` does not exist yet, create the smallest bootstrap commit needed to establish it, push `main`, and then move normal work to a topic worktree.

Do not force-push task branches. If push is rejected because the remote branch changed, inspect the remote changes and bring required adjustments in with a new commit.

## Child Worktree Thread Titles

When a task lane is delegated to a forked Codex worktree child thread, rename
the child thread after setup completes and a thread id exists. Use
`set_thread_title` when that tool is available.

Thread titles should include the project, issue number, and lane purpose, for
example `Codexy #52 refactoring skill agent lane`.

If thread title renaming is unavailable, mention that limitation in the parent
status or child handoff and continue the lane. A missing title rename is a
clarity issue, not a merge blocker for otherwise complete implementation work.

## Local Change Discipline

Inspect before editing or committing:

```sh
git status --short
git diff
```

Stage only intended files. Preserve unrelated dirty work. Do not revert or discard user changes unless the user explicitly asks for that exact operation.

Do not commit `.omo/**` evidence, local logs, secrets, or scratch files by default. Reference local evidence paths and summarized results in the PR body.

## Commit Messages

Use Conventional Commit style:

```text
<type>(<scope>): <summary>
```

Common types:

| Type | Use for |
| --- | --- |
| `feat` | User-visible feature or workflow |
| `fix` | Bug fix |
| `docs` | User-facing documentation only |
| `refactor` | Behavior-preserving restructure |
| `test` | Test additions or updates |
| `chore` | Maintenance, repository setup, or agent workflow changes |
| `ci` | CI or automation workflow changes |
| `revert` | Reverting an earlier change |

Project-local skill changes under `plugins/codexy/skills/**` change agent behavior, so prefer a non-`docs` type such as `chore`, `feat`, `fix`, or `refactor`.

Avoid vague messages such as `update`, `fix`, `WIP`, or `misc`.

## Verification Before Push Or PR

Run verification that covers every touched surface before claiming completion, pushing, or opening/updating a PR.

For docs, license, and workflow-only changes, use focused checks such as:

```sh
git diff --check
test -f README.md
test -f LICENSE
test -f AGENTS.md
test -f plugins/codexy/skills/git-workflow/SKILL.md
git check-ignore .omo/ulw-loop/example
```

For code changes, add the relevant lint, typecheck, unit, integration, harness, or end-to-end commands once the repository has those surfaces.

When the requested behavior is a GitHub setting, branch rule, PR lifecycle, CLI, browser page, desktop app, or other external surface, drive that surface directly and capture observable evidence. Tests alone are supporting evidence, not completion proof.

## Pull Requests

Open PRs with GitHub or `gh`. Keep PRs draft only while local verification is missing or risk is intentionally unresolved.

Create or confirm a GitHub issue before opening a PR unless a maintainer explicitly scopes an issue-free exception in the current thread.

PR titles must use Conventional Commit style:

```text
chore(repo): establish repository governance
```

PR bodies must be structured with Markdown headings. Do not create placeholder, one-line, or notes-only PR bodies.

PR bodies must include:

- `## Summary`: what changed.
- `## Rationale`: why it changed.
- `## Changed Areas`: changed files or areas.
- `## Verification`: verification commands run and results.
- `## Evidence`: local evidence paths and key pass/fail results when ULW or manual QA evidence exists.
- `## Not Run`: any verification that could not be run and why, or `None`.
- `## Follow-ups`: known follow-ups, or `None`.

When a matching issue exists, put the closing reference only on the final line:

```text
Fixes #<issue-number>
```

Do not put closing references in the middle of the PR body.

When labels are available, inspect the repository's current label taxonomy
before opening or updating a PR. Apply repository-appropriate labels to the PR
using the same taxonomy principles as issues, without hard-coding a fixed label
list. If the repository uses status-like labels, keep issue and PR labels
aligned with state transitions such as review requested, review feedback routed,
merge, close, or reopen.

## Codex Review Gate

Codex connector review is a real merge gate when it is expected for the
repository or when the maintainer asks for it. Opening a PR is not completion,
and merging is not completion if actionable Codex feedback has not been checked
and handled.

After opening or updating a PR, inspect Codex review state on the latest head.
Do not rely only on GitHub review objects. `chatgpt-codex-connector` can deliver
actual review results as inline review comments, GitHub review objects, or
top-level PR issue comments.

```sh
gh pr view <pr> --json number,headRefOid,reviews,latestReviews,comments,reviewDecision,statusCheckRollup
gh api repos/<owner>/<repo>/pulls/<pr>/comments --paginate
gh api repos/<owner>/<repo>/issues/<pr>/comments --paginate
```

Identify Codex connector output by any of these GitHub API signals:

- `performed_via_github_app.slug == "chatgpt-codex-connector"`.
- `user.login == "chatgpt-codex-connector[bot]"` or the compact PR view author appears as `chatgpt-codex-connector`.
- The GitHub App avatar/icon URL belongs to the `chatgpt-codex-connector` app user.

These identity signals only prove the connector authored the comment or review.
They do not prove the review completed; interpret the comment body and reaction
state before treating it as a merge gate pass.

If the expected automatic Codex review does not appear after a reasonable wait,
request it explicitly with a PR comment:

```sh
gh pr comment <pr> --body "@codex review"
```

An `eyes` reaction on the `@codex review` comment means Codex noticed the request
and is processing it. It is not approval and does not mean review is complete.
When an in-progress `@codex review` request already has an `eyes` reaction for
the current PR head, do not send duplicate review requests for that same head.
Keep polling PR comments, reviews, and review threads instead. If the request
appears stale for an unusually long time, document the status and either keep
polling or escalate once with a distinct rationale; do not issue repeated blind
`@codex review` comments.

Codex review completion signals include:

- Inline review comments or review suggestions from `chatgpt-codex-connector`; these are complete review output, but actionable comments block merge until fixed or explicitly accepted by a human maintainer.
- A top-level PR comment from `chatgpt-codex-connector` that contains actual review results, suggestions, or no-issue/no-suggestion wording; this is also Codex review output, even when no GitHub review object appears.
- A Codex comment such as `Didn't find any major issues` or equivalent no-suggestion wording; this means the reviewed head has no major actionable suggestions.
- A Codex thumbs-up/no-suggestion result, such as `+1` or a thumbs-up reaction, when no inline suggestions are produced; this is acceptable only after confirming it applies to the latest PR head.

Setup or environment comments, such as `create an environment for this repo`,
are connector responses but not review content and not review completion. Treat
them as infrastructure blockers unless the maintainer explicitly accepts
proceeding without a full Codex review.

If any new commits are pushed after Codex review, the old review no longer
proves the current head. Wait for or request a fresh Codex review before
merging. After a fresh-review request for that new head receives `eyes`, the
correct action is waiting and polling until review output appears, not repeated
requests for the same head.

### Child-Owned Review Feedback

When a PR was produced by a delegated child Codex worktree thread, the
plugin-invoking parent thread is the orchestrator, not the implementation
worker for that lane.

- The child thread owns implementation edits, local verification, and
  review-response fixes for its assigned issue-sized lane.
- If Codex connector or human review feedback flags a child-owned PR, the
  parent MUST route the feedback back to the owning child thread instead of
  directly patching the branch.
- The parent handoff must include the PR number, latest head SHA, relevant
  comments or review thread URLs, allowed files, expected return evidence, and
  stop condition.
- The parent may make implementation edits only for its own explicitly scoped
  lane, or when a maintainer explicitly overrides the boundary and reassigns
  the lane to the parent.
- The parent may resolve review threads only after child evidence proves the
  fix on the current head, or after a maintainer accepts a no-change rationale.
- Worktree lanes must remain issue-sized and atomic. Do not combine review
  feedback from one child lane with another branch or PR.

## Repository Settings And Main Protection

Repository settings should keep:

- `main` as the default branch.
- Squash merge enabled.
- Merge commits disabled.
- Rebase merge disabled.
- Delete branch on merge enabled.

`main` should block direct updates and require pull requests before changes land. Use repository rulesets or classic branch protection when the GitHub plan and repository visibility allow them. If GitHub rejects protection because the private repository lacks the required plan, report the exact platform blocker and do not make the repository public unless the maintainer explicitly approves that visibility change.

## Merge Rules

Do not merge a PR until every review surface has been inspected and resolved.
Codex connector reviews are merge-blocking reviews. Treat them the same as a
human maintainer review: requested changes, actionable suggestions, unresolved
review threads, stale concerns after new commits, and PR comments that identify
defects must be addressed with code, documentation, or a clearly documented
non-change rationale before merge.

### Known Merge Subject Deviation

PR #18 was squash merged as `docs(license): correct copyright owner (#)` because
the merge command did not carry the numeric PR identifier into the subject. Do
not rewrite protected `main` history to repair that old commit. Treat it as a
recorded process deviation and prevent repeats by deriving the PR number from an
explicit `gh pr view <number>` call before every merge.

Before merging, inspect the latest PR state, checks, reviews, comments, and
review threads:

```sh
gh pr view <pr> --json number,title,state,headRefName,headRefOid,baseRefName,mergeStateStatus,statusCheckRollup,reviewDecision,latestReviews,reviews,comments
gh pr view <pr> --comments
gh api graphql -f owner=<owner> -f name=<repo> -F number=<pr-number> -f query='
query($owner:String!, $name:String!, $number:Int!) {
  repository(owner:$owner, name:$name) {
    pullRequest(number:$number) {
      reviewThreads(first:100) {
        nodes {
          isResolved
          isOutdated
          comments(first:20) {
            nodes {
              author { login }
              body
              url
            }
          }
        }
      }
    }
  }
}'
```

The review gate is satisfied only when:

- `reviewDecision` is not `CHANGES_REQUESTED`.
- No latest review from a maintainer, GitHub app, or Codex connector requests changes.
- The expected Codex review has completed on the latest `headRefOid`; if it was missing, `@codex review` was requested and its completion signal was confirmed.
- Every non-outdated review thread is resolved, or the PR body/comment history documents why no change is required.
- Every actionable PR comment has been addressed or explicitly marked non-actionable with rationale.
- You have re-run verification after addressing review feedback.

If any review or comment is ambiguous, stop and resolve it before merging. Do
not merge first and plan to address review feedback afterward.

When the PR satisfies the merge gates, merge through GitHub with squash merge
and branch deletion. Prefer `--match-head-commit <headRefOid>` when available so
a newly pushed unreviewed head cannot be merged by accident:

```sh
pr_number=<explicit-pr-number>
gh pr view "$pr_number" --json number,headRefOid,title
gh pr merge "$pr_number" --squash --delete-branch --match-head-commit <headRefOid> --subject "<conventional subject> (#${pr_number})"
```

`gh pr merge` does not have a flag that means "Codex review passed." `--auto`
only waits for requirements configured in GitHub, and `--admin` bypasses
requirements. Do not use `--admin` to skip Codex review, required checks, or
review-thread cleanup.

Do not locally merge feature branches into `main` as a substitute for the PR workflow.

After merge, update the main worktree:

```sh
git pull --ff-only origin main
git log -1 --pretty=%s
```

The refreshed `main` commit subject must end with `(#<merged-pr-number>)`. If GitHub did not delete the remote topic branch, delete it only after confirming the PR was merged and no dependent work needs the branch:

```sh
git push origin --delete <branch>
```

## Conflict Resolution

Before resolving conflicts, inspect the state:

```sh
git status
git diff
```

Resolve conflict markers carefully. Preserve both sides' intended behavior when possible. If the correct resolution depends on domain intent, stop and ask before editing.

After resolving, stage only the resolved files and run verification relevant to the conflict surface.

## Quick Checklist

- Issue exists or a maintainer provided an explicit issue-sized scope.
- Issue and PR labels match the repository's current label taxonomy when labels
  are available; status-like labels have been updated after review, merge,
  close, or reopen transitions.
- Branch is not `main`, uses the requested prefix, and lives in an isolated worktree.
- Branch scope matches the issue or sub-scope.
- Local `.omo/**` evidence remains uncommitted unless explicitly requested.
- No force push or force-with-lease is used.
- Verification covers touched surfaces.
- PR body has structured sections and ends with exactly one `Fixes #<issue-number>` line when a matching issue exists.
- Expected Codex review completed on the latest PR head, and no unresolved actionable Codex feedback remains.
- PR reviews, Codex connector reviews, PR comments, and review threads have been inspected and all actionable feedback is resolved or explicitly documented as non-actionable before merge.
- Repository merge settings allow squash only and delete branches after merge.
- Main protection is configured, or an exact GitHub plan/visibility blocker is documented.
- Merge is squash merge through the PR workflow.
- The final squash merge subject ends with the PR number.
- The remote branch is deleted after merge.
- The main worktree is refreshed with `git pull --ff-only origin main`.
