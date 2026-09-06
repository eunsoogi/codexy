---
name: git-workflow
description: Use for GitHub issue, branch, worktree, pull request, review, merge, CI, and release work in any repository under the public Codexy orchestration contract.
---

# Git Workflow

MUST use this skill with `$orchestration` before any GitHub workflow action.
This skill applies to GitHub work in any repository. Repository-local
requirements come only from user direction, governing `AGENTS.md`, and
authenticated live GitHub state; this skill owns only GitHub-specific
authenticated admission and lifecycle boundaries.

## Read The Matching Reference

- Issue creation: [issue-intake.md](references/issue-intake.md)
- Branch, worktree, commit, or conflict work:
  [local-git-and-branches.md](references/local-git-and-branches.md)
- PR creation, readiness, review, or child handoff:
  [pr-review-and-handoff.md](references/pr-review-and-handoff.md)
- Repository-required Codex review:
  [codex-connector-review.md](references/codex-connector-review.md)
- Merge authorization:
  [merge-authorization.md](references/merge-authorization.md)
- Squash merge and main sync:
  [merge-and-main-sync.md](references/merge-and-main-sync.md)

MUST read only the references matching the requested operation before acting.

## Admission Boundaries

- For issue-sized implementation work, or when a repository or maintainer
  explicitly selects an issue-owned branch/worktree process, MUST confirm the
  issue or scoped exception and keep one isolated owner branch/worktree aligned
  to it. Ordinary authorized GitHub metadata and remote operations do not
  require an issue, local branch, worktree, or plugin-owned preparation merely
  because this component is installed.
- MUST read the configured default branch and protection before branch or
  worktree setup. MUST NOT implement on that branch or force-push a task branch.
- MUST read current repository, target, PR, base, head, checks, reviews,
  comments, labels, issue linkage, and review threads before a readiness or
  handoff claim that uses this evidence.
- MUST route child-owned review fixes to the owning child when that lane exists.
  Unresolved actionable feedback remains blocking for the selected review or
  handoff contract.
- MUST inspect the live repository taxonomy before issue or PR label mutations
  when labels are part of the requested operation.

The installed plugin adds workflow context and narrowly scoped local safety
checks. It does not admit, deny, or rewrite GitHub mutations. Commands MUST use
the host, connector, and GitHub authorization that applies to the current
session; repository-local instructions remain repository-owned.

## Merge Boundary

Merge and auto-merge mutations use the normal host or connector route and
GitHub's server-side permissions and branch protections. Gate success, generic
completion, local state, or parent prose does not manufacture authorization.

## GitHub And Local Tools

Prefer authenticated GitHub connector reads for issue, PR, review, thread,
branch, commit, and status evidence. When the optional connector is unavailable,
MUST use authenticated `gh` reads for the same read-only evidence. Use local
`git` for worktree state, diffs, staging, commits, and ordinary pushes. A
required read surface unavailable through both connector and `gh` MUST fail
closed; local prose, fixtures, or mocks are not substitutes for live GitHub
evidence.

Before push or PR readiness, MUST run verification for every touched surface,
`git diff --check`, clean-scope status/diff inspection, the repository's public
touched-file LOC check, and relevant package validation. External behavior also
requires matching live GitHub readback.
