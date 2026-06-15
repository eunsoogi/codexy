# Agent Instructions

## Project

Codexy is a Codex harness and loop engineering repository. It is used for agent execution loops, verification harnesses, evidence capture, workflow automation, and small tools that improve Codex work quality.

## Scope

- This file governs the whole repository.
- Keep broad repository guidance in this root `AGENTS.md`.
- Add nested `AGENTS.md` files only when a subtree has stable, local rules that should not apply elsewhere.
- If this file conflicts with a deeper `AGENTS.md`, the deeper file wins inside its subtree.

## Documentation

- `README.md` is the concise English first-user introduction.
- `README.ko.md` is the concise Korean first-user introduction.
- Keep both README files scoped to the current implemented state of the project.
- `LICENSE` must remain the standard English MIT license text.
- Keep executable Git, issue, PR, and merge rules in `plugins/codexy/skills/git-workflow/SKILL.md`.

## Workflow

- Keep `main` as the protected integration branch.
- Do not implement feature, fix, cleanup, documentation, workflow, or UX work directly on `main`.
- Direct pushes to `origin/main` are allowed only for one-time bootstrap work that is impossible before `origin/main` exists, or when a maintainer explicitly requests that exact operation.
- Start every normal task from a GitHub issue or a maintainer-provided issue-sized scope.
- Use an isolated git worktree with a topic branch for each task.
- Use the `codexy/` branch prefix unless the user requests another naming scheme.
- Keep each branch tied to one issue-sized outcome.
- Split unrelated follow-ups into separate issues and PRs.
- Use Conventional Commit style for commit titles and PR titles.
- Push topic branches with ordinary push only. Do not force-push unless a maintainer explicitly asks for history rewriting.
- Merge completed work through Pull Requests.
- Use squash merge only.
- Squash merge commit subjects on `main` must keep Conventional Commit style and end with the PR number, for example `chore(repo): establish governance (#1)`.
- Delete the remote topic branch after a PR is merged.
- After merge, refresh the main worktree with `git pull --ff-only origin main`.

## Git Workflow Skill

- Before creating or updating issues, branches, worktrees, commits, pushes, PRs, labels, reviews, branch protection, repository settings, merges, or post-merge sync state, use the project-local `git-workflow` skill from `plugins/codexy/skills/git-workflow/SKILL.md`.
- If the skill conflicts with this file, this file wins.
- Use GitHub and `gh` for GitHub state when no richer connector is active.
- Use local `git` for local worktree inspection, staging, committing, and ordinary pushes.

## Verification

- Run verification that covers every touched surface before pushing or opening a PR.
- For documentation and workflow-only changes, at minimum run:
  - `git diff --check`
  - file existence checks for changed workflow files
  - a rendered or parsed inspection when the change is structured data
- For code changes, add project-specific lint, typecheck, unit, integration, or harness commands before claiming the work is complete.
- Tests alone do not prove user-visible completion when the requested surface is a CLI, GitHub repository setting, browser page, desktop app, or other externally observable workflow. Drive the matching surface and capture evidence.

## Local State

- Do not commit `.omo/**` ULW state, temporary evidence, local logs, machine-specific credentials, or generated scratch files unless a maintainer explicitly requests a tracked artifact.
- Do not store GitHub tokens, Codex credentials, API keys, private logs, or local machine paths in tracked files.

## Style

- Prefer small, surgical changes that directly satisfy the issue.
- Do not add speculative framework, package, or workflow assumptions.
- Mention unrelated stale work instead of cleaning it up inside the current PR.
- Keep instructions actionable: use `MUST` or `MUST NOT` only for hard requirements.
