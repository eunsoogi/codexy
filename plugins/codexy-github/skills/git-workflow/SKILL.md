---
name: git-workflow
description: Use for GitHub issue, branch, worktree, pull request, review, merge, CI, and release workflow with the public Codexy orchestration contract.
---

# Git Workflow

MUST use this skill with the public `$orchestration` contract before GitHub issue, branch, worktree, commit, push, PR, review, repository-settings, branch-protection, merge, or post-merge sync work.

## Read Next

MUST read these relative references before acting on the matching surface:

- `references/local-git-and-branches.md` for branch/worktree setup, local change discipline, commit messages, conflict resolution, and pre-PR Git checks.
- `references/issue-intake.md` before any Codexy-created GitHub issue mutation.
- `references/pr-review-and-handoff.md` for PR bodies, review-thread handling, child-owned review feedback, and completion-handoff PR state capture, including review thread comment `commit { oid }`
  evidence.
- `references/codex-connector-review.md` for the one explicit pre-merge Codex connector review and its bounded repair cycle.
- `references/merge-and-main-sync.md` and `references/merge-authorization.md` for merge gates, squash merge body preservation, branch deletion, post-merge main sync, and the
  `merge_validation_args=(--check-merge-message --expected-pr "$pr_number")` / `post_merge_validation_args=(--check-merge-message --expected-pr "$pr_number")` guards.

## Authority

The active repository's `AGENTS.md`, direct user instructions, and GitHub issue scope define local policy. This plugin supplies generic workflow only; it MUST NOT package or override
repository-specific GitHub policy or workflow files.

MUST use GitHub and `gh` for issue, pull request, review, check, label, branch-protection, repository-settings, and merge state when connector tools are not already handling that surface. MUST use
local `git` for local worktree inspection, checkout, worktree creation, diff, staging, committing, rebasing, pulling, and ordinary push.

## Start Work

1. MUST read `AGENTS.md` and this skill.
2. MUST use `$orchestration` before issue setup, branch/worktree setup, delegation, implementation, PR handling, review-response routing, merge coordination, or validation-only work begins. MUST keep
   classification evidence in the thread or handoff.
3. MUST create or confirm a GitHub issue before implementation. If the user provided an issue, treat that issue as the source of truth.
4. For non-trivial work, MUST keep a short plan and update it as evidence changes.
5. MUST discover the repository's configured default and protected integration branch. MUST NOT implement directly on that branch.
6. MUST create a branch only after the issue or explicit issue-sized scope exists.
7. MUST use an isolated git worktree for the task branch.
8. MUST use the repository's configured branch naming convention, or a user requested naming scheme when no repository policy applies.
9. MUST keep the branch scope aligned with the issue.

When this plugin is directly installed, Codex discovers its `git-workflow` skill, Weaver agent, and host-resolved `hooks/hooks.json` package. The generic admission hooks are activated by Codex with
`${PLUGIN_ROOT}`; skill-authored commands MUST NOT derive a source-checkout, cache, or ambient executable path. The optional `codexy-github-install` tool MAY coordinate a getcodexy component
transaction, but MUST NOT be required to use this installed plugin.

Issue titles MUST summarize the user-visible problem or needed work in plain prose. They MUST start with an uppercase letter and MUST NOT use Conventional Commit prefixes such as `feat(...)`. The
installed generic admission hooks enforce issue-title and related workflow requirements on their matching host lifecycle events. Capture the exact remote title and treat hook context as advisory only;
MUST NOT construct a repository, source-checkout, cache, or ambient executable path to bypass the package.

Issue bodies MUST include `## Problem`, `## Scope`, `## Acceptance Criteria`, and `## Verification`.

Before any Codexy-created issue mutation, child lanes MUST ask the installed `$orchestration` skill to apply its **issue-intake receipt** contract, submit that canonical JSON receipt to the parent,
and receive explicit approval. The receipt MUST follow `references/issue-intake.md`. Unsupported synthetic wording and same-class phrase variants are handoff-only.

When labels are available, MUST inspect the repository's current taxonomy before creating or updating issues. MUST apply repository-appropriate labels only when those concepts exist.

## Worktrees And Branches

Before branch, worktree, local commit, push, or conflict work, MUST read `references/local-git-and-branches.md`. MUST start task branches from the current configured default branch, MUST NOT work
directly on it, and MUST NOT force-push task branches.

## Child Worktree Thread Titles

For forked Codex worktree child threads, the orchestrator MUST rename the child thread after setup and thread id availability with `set_thread_title` when that tool is available. Thread titles MUST
include the project, issue number, and lane purpose, for example `Codexy #52 refactoring skill agent lane`. If renaming is unavailable, mention that limitation in parent status or child handoff; a
missing title rename is not a merge blocker for otherwise complete implementation work.

## Local Change Discipline

MUST read `references/local-git-and-branches.md` before staging or committing. MUST inspect `git status --short` and `git diff`, stage only intended files, and MUST NOT revert or discard user changes
unless explicitly asked.

## Commit Messages

MUST read `references/local-git-and-branches.md` before committing. Commit messages MUST use Conventional Commit style and MUST NOT use vague subjects such as `update`, `fix`, `WIP`, or `misc`.

## Verification Before Push Or PR

MUST run verification that covers every touched surface before claiming completion, pushing, or opening/updating a PR.

For docs, license, and workflow-only changes, MUST use focused checks such as:

```sh
git diff --check
test -f README.md
test -f LICENSE
test -f AGENTS.md
git check-ignore .omo/ulw-loop/example
```

For non-trivial code, validator, harness, workflow-rule, or skill instruction changes, MUST run:

```sh
the active repository's public touched-file LOC validation command
```

MUST treat every governed file over the 250 LOC target as review-blocking. Every governed file MUST stay at or below 250 LOC. MUST NOT use or authorize LOC exceptions. MUST treat a formatting-only
reduction as review-blocking: blank-line deletion or collapsed readable multiline code, tests, or instructions does not prove structural LOC remediation.

When the requested behavior is a GitHub setting, branch rule, PR lifecycle, CLI, browser page, desktop app, or other external surface, MUST drive that surface directly and capture observable evidence.
Tests alone are supporting evidence, not completion proof.

For code-touching or code-adjacent runtime changes, MUST use Codexy `codegraph` MCP when available and confirm exact files with direct reads. For language-aware code edits, MUST use Codexy `lsp` when
callable, or include `lsp_status` evidence.

## Pull Requests

MUST open PRs with GitHub or `gh`. MUST keep PRs draft only while local verification is missing or risk is intentionally unresolved. MUST create or confirm a GitHub issue before opening a PR unless a
maintainer explicitly scopes an exception.

PR titles MUST use Conventional Commit style, such as `chore(repo): repository governance`. Capture the GitHub API value and let the installed generic admission hooks enforce their matching lifecycle
checks. MUST NOT treat `UserPromptSubmit` advisory context as PR title, PR label, or merge-message enforcement.

PR bodies MUST include `## Summary`, `## Rationale`, `## Changed Areas`, `## Verification`, `## Evidence`, `## Not Run`, and `## Follow-ups`. When a matching issue exists, put the closing reference
only on the final line:

```text
Fixes #<issue-number>
```

When labels are available, MUST inspect the current taxonomy before opening or updating a PR. MUST apply repository-appropriate labels before or immediately after PR creation without hard-coding a
fixed list. PR-readiness handoff is valid only when captured PR state shows labels, or repository label taxonomy proves none exist. Before PR readiness, MUST preserve the captured PR state for the
installed generic admission hooks and evidence handoff.

Before merge, the parent/orchestrator MUST follow the manual Codex connector review procedure in `references/codex-connector-review.md`.

## Child-Owned Review Feedback

When a PR was produced by a delegated child Codex worktree thread, the plugin-invoking parent thread is the orchestrator, not the implementation worker for that lane.

- The child thread owns implementation edits, local verification, and review-response fixes for its assigned issue-sized lane.
- For any lane that needs its own branch, worktree, PR, or durable child context, the parent MUST create, fork, or assign the child thread before implementation patches begin. The parent MUST NOT make
  draft implementation edits first and delegate afterward.
- Subagents are not child-owned implementation owners, and `codex exec`, `codex fork`, or generic `codex app-server` commands MUST NOT be claimed as fallback substitutes for a required Codex
  thread/worktree owner.
- For non-trivial lanes, the child thread MUST report actual goal tool usage, actual todo/plan tool usage, multi-agent usage or a concrete not-useful rationale, codegraph evidence, LSP status
  evidence, and unavailable-tool fallbacks.
- Before returning a non-trivial atomic lane as ready, the owning thread MUST follow the public `$orchestration` review-profile contract: light has no LLM reviewer, standard has Inspector, and strict
  has Sentinel.
- If human or automated review feedback flags a child-owned PR, the parent MUST route the feedback back to the owning child thread instead of directly patching the branch.
- If the owning child thread is unresponsive or is unable to return evidence, the parent MUST stop and report the blocker, current PR head, child owner, last contact, and required next evidence. The
  parent MUST NOT patch the child-owned branch as recovery unless there is explicit maintainer reassignment.
- Before accepting evidence that mentions parent-authored implementation or review-response commits, MUST ask `$orchestration` to apply its **child-lane-ownership** contract.

## Repository Settings And Main Protection

Repository settings and merge policy are repository-owned. MUST inspect active repository policy, obtain explicit authority before a settings mutation, and report any platform limitation without
attempting a policy substitution.

## Conflict Resolution

Before resolving conflicts, MUST read `references/local-git-and-branches.md`. MUST preserve both sides' intended behavior when possible, MUST stop and ask when domain intent is unclear, and MUST stage
only resolved files.

MUST read [`references/quick-checklist.md`](references/quick-checklist.md) before declaring GitHub workflow readiness.
