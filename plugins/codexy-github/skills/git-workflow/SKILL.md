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

- MUST confirm an issue or explicit maintainer-scoped exception before
  implementation, then keep one isolated owner branch/worktree aligned to it.
- MUST read the configured default branch and protection before setup. MUST NOT
  implement on that branch or force-push a task branch.
- MUST read current repository, target, PR, base, head, checks, reviews,
  comments, labels, issue linkage, and review threads before a readiness claim.
- MUST route child-owned review fixes to the owning child. Unresolved actionable
  feedback remains blocking.
- MUST inspect the live repository taxonomy before issue or PR label mutations.

Installed generic hooks enforce their matching issue, PR, and merge admission
events. Commands MUST NOT derive source-checkout, cache, ambient executable, or
`${PLUGIN_ROOT}` paths to bypass the installed package.

## Merge Boundary

Direct or nested connector merge and auto-merge mutations are `UNAVAILABLE`. The
only Codexy-owned merge entrypoint is the installed host-resolved resource
`skills/git-workflow/scripts/codexy-authorized-squash-merge.sh`. It fresh-reads
and validates the exact GitHub authorization and delegates to the canonical
hooked wrapper. Gate success, generic completion, local state, or parent prose
does not authorize merge.

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
