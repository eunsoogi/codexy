---
name: proof-driven-completion
description: Use before claiming work is done, opening or merging a PR, handing off, closing an issue, reporting success, or completing a goal for code, docs, workflow, UI, plugin, marketplace, or release tasks.
---

# Proof-Driven Completion

## Purpose

Completion is a claim about the current state, not a feeling about effort. This
skill requires evidence that directly matches every explicit requirement before
the agent says work is done, closes an issue, merges a PR, or marks a goal
complete.

## Completion Audit

1. Restate the requested outcome.
2. List every explicit requirement, named file, command, review gate, external
   state, and deliverable.
3. For each item, name the evidence that would prove it:
   - file content or diff for documentation and configuration,
   - parser/schema output for structured data,
   - lint/typecheck/unit/integration output for code,
   - browser, desktop, CLI, GitHub, plugin, or marketplace observation for
     user-visible or external behavior,
   - PR review/comment/thread state for review gates.
4. Inspect the current authoritative source. Do not rely on memory, intent, or
   earlier output unless it is explicitly marked as stale supporting context.
5. Classify each item as proved, contradicted, incomplete, too weak, or missing.
6. Continue work until every required item is proved, or report the exact
   blocker without calling the task complete.

## Required Checks

- Run `git diff --check` before pushing or opening a PR.
- Inspect `git status --short` and avoid staging unrelated files.
- Parse structured files with an appropriate parser when possible.
- For plugin skills, confirm every `SKILL.md` has valid YAML frontmatter with
  `name` and `description`.
- For GitHub PR work, inspect PR state, latest head SHA, comments, reviews,
  review threads, and Codex connector output on the current head.
- Re-run verification after addressing review feedback.
- For delegated non-trivial or multi-step child implementation lanes, verify
  the child reported its own goal state or fallback, current todo/plan status,
  multi-agent use or unavailable-tool fallback, changed files, verification
  evidence, and clean worktree status before treating the handoff as complete.
  For an atomic trivial child lane, require an explicit not-applicable rationale
  instead of silently skipping the execution discipline.

## Evidence Rules

- Evidence must be current for the file state, commit, PR head, runtime, or
  external setting being claimed.
- Narrow evidence proves only narrow claims. A parser check does not prove UX; a
  unit test does not prove GitHub settings; an `eyes` reaction does not prove
  Codex review completion.
- If new commits land after review, request or wait for fresh review on the new
  head.
- If a command was skipped, say so with the reason.
- If evidence is local and untracked, summarize it or give the ignored evidence
  path; do not commit scratch artifacts unless requested.

## Final Report Shape

Include:

- outcome,
- changed files or surfaces,
- verification commands and results,
- external observations such as PR review state or UI behavior,
- not run,
- blockers or residual risks.

## Stop Conditions

- Stop and fix if proof contradicts the claim.
- Stop and ask only when the missing proof requires a secret, account action,
  destructive operation, or human-only decision.
- Do not call `update_goal(status="complete")` until every requirement has
  current matching proof and no required work remains.
- Do not accept a non-trivial child implementation handoff as complete when it
  omits goal, todo/plan, or multi-agent/fallback evidence required by the
  orchestrator assignment.

## Failure Modes

- Reporting a merge before verifying branch deletion and main sync.
- Ignoring Codex connector comments because they are top-level PR comments
  instead of GitHub review objects.
- Treating generated files as valid without parsing or inspecting them.
- Forgetting cleanup of worktrees, sessions, ports, temp logs, or stale
  evidence.
