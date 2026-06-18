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
   - `scripts/validate-plugin-config --check` for Codexy plugin
     architecture surfaces when the validator exists,
   - lint/typecheck/unit/integration output for code,
   - browser, desktop, CLI, GitHub, plugin, or marketplace observation for
     user-visible or external behavior,
   - PR review/comment/thread state for review gates.
   - child-thread handoff or readback evidence when feedback belongs to a
     child-owned lane.
4. Inspect the current authoritative source. Do not rely on memory, intent, or
   earlier output unless it is explicitly marked as stale supporting context.
5. Classify each item as proved, contradicted, incomplete, too weak, or missing.
6. Continue work until every required item is proved, or report the exact
   blocker without calling the task complete.

## Required Checks

- Run `git diff --check` before pushing or opening a PR.
- Inspect `git status --short` and avoid staging unrelated files.
- Parse structured files with an appropriate parser when possible.
- For Codexy plugin architecture changes, validate LSP config, MCP config,
  role metadata or custom agent TOMLs, and thread/worktree orchestration
  wording. Run `scripts/validate-plugin-config --check` when that
  script is present in the current revision.
- For code-touching or code-adjacent runtime changes, include Codexy
  `codegraph` MCP exploration evidence when the MCP is available, plus direct
  file-read confirmation before claiming the touched surface is understood.
- For non-trivial code, validator, harness, or workflow-rule changes, run a
  touched implementation-file LOC gate before PR readiness or handoff:
  `scripts/validate-plugin-config --check-touched-loc --base-ref <base>`.
  Treat files over the 250 LOC target as failing evidence unless the tracked
  Codexy LOC exception mechanism names the file and rationale. Handoff or PR
  body prose alone is not proof of an exception.
- For plugin skills, confirm every `SKILL.md` has valid YAML frontmatter with
  `name` and `description`.
- For GitHub PR work, inspect PR state, latest head SHA, comments, reviews,
  review threads, and Codex connector output on the current head.
- For child-owned PRs, route actionable review feedback back to the owning
  child thread. The parent thread may coordinate, but it must not merge until
  the child thread returns current verification or a documented non-change
  rationale.
- If a child-owned PR handoff or final-answer evidence mentions parent-authored
  implementation or review-response commits, run
  `scripts/validate-plugin-config --check-child-lane-ownership --evidence-file <path>`.
  A failing result blocks completion unless the evidence records explicit
  maintainer reassignment of implementation ownership to the parent.
- For delegated lanes that need their own branch, worktree, PR, or durable
  child context, require evidence that the child was created, forked, or
  assigned before implementation patches began. If parent-authored draft edits
  exist, require recovery evidence showing the parent stopped editing,
  disclosed the mistake, protected user and other-agent work, and handed the
  draft diff to the owning child thread.
- For every non-trivial atomic unit, require evidence that the owning thread
  ran the packaged Codexy reviewer agent defined by
  `plugins/codexy/agents/codexy-sentinel.toml` before handoff, PR readiness,
  completion, or parent acceptance. The reviewer gate must cover the current
  diff, exact head or file state, lane scope, touched implementation-file LOC
  evidence, verification outputs, and evidence. Arbitrary reviewer agents,
  generic role names, parent-only
  readthroughs, stale reviewer output, or external review passes are not
  substitutes for this gate.
- Re-run verification after addressing review feedback.
- For delegated non-trivial or multi-step child implementation lanes, verify
  the child reported actual goal-tool usage or an unavailable-goal-tool
  fallback, current todo/plan tool usage or an unavailable-todo-tool fallback,
  required multi-agent use for independent research questions, disjoint
  implementation slices, QA or verification in parallel, review gates,
  review-feedback validation, or separable non-trivial subtasks, changed
  files, verification evidence, packaged Codexy reviewer findings or approval,
  and clean worktree status before treating the handoff as complete. A
  "multi-agent not useful" rationale is acceptable only when it is concrete
  and tied to atomicity, tiny scope, or the absence of separable work; generic
  manual fallback is not enough when multi-agent tooling is available.
  Goal-tool evidence should name real Codex goal surfaces such as
  `create_goal`, `get_goal`, or `update_goal` when they are available.
  Prose-only `Goal:` or `Todo:` text is not evidence of real goal or todo/plan
  tool use.
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
- If review feedback is addressed by a child thread, evidence must include the
  child thread result, the exact new head, and the rerun verification.
- If a fresh `@codex review` request for the current head already has `eyes`,
  do not send duplicate requests for the same head. Continue polling and
  waiting for review output. If it is unusually stale, document the status and
  use a distinct escalation rationale instead of repeated blind requests.
- If a command was skipped, say so with the reason.
- If evidence is local and untracked, summarize it or give the ignored evidence
  path; do not commit scratch artifacts unless requested.
- If a dependency PR has not yet landed, label validator, LSP, MCP, role
  metadata, custom agent TOML, thread, or worktree evidence as deferred instead
  of claiming completion.

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
- Do not call `update_goal(status="blocked")` merely because Codex connector
  review, child-thread work, queued worktree/thread setup, or asynchronous tool
  completion is pending. Continue polling, send follow-up prompts as needed,
  route review feedback to the owning child thread, and keep the goal active
  until a repeated true impasse prevents meaningful progress without user input
  or an external state change.
- Do not accept a non-trivial child implementation handoff as complete when it
  omits actual goal-tool usage, actual todo/plan tool usage, required
  situational multi-agent usage, a concrete not-useful rationale tied to
  atomicity or tiny scope, or unavailable-tool fallback evidence required by
  the orchestrator assignment.
  Using only one of goal or todo/plan is insufficient unless the missing tool
  was unavailable and the child reported that unavailability with its fallback.
- Do not accept a non-trivial atomic unit as complete when it omits the
  packaged Codexy reviewer agent result for the current diff, exact head or
  file state, lane scope, verification outputs, and evidence.

## Failure Modes

- Reporting a merge before verifying branch deletion and main sync.
- Ignoring Codex connector comments because they are top-level PR comments
  instead of GitHub review objects.
- Treating ordinary review or child-thread wait time as a blocker instead of an
  active goal state.
- Treating generated files as valid without parsing or inspecting them.
- Forgetting cleanup of worktrees, sessions, ports, temp logs, or stale
  evidence.
- Treating prose about architecture gates as proof that LSP, MCP, role
  metadata, custom agent TOML, thread, or worktree behavior has been validated.
- Treating code-touching work as complete without Codexy `codegraph` MCP
  exploration evidence when the MCP was available.
- Fixing child-owned review feedback in the parent thread and merging without
  handing it back to the owning child thread for verification.
- Accepting child-owned lane completion when the parent patched implementation
  first and delegated afterward without explicit recovery evidence.
- Treating an arbitrary reviewer agent, generic review role, parent-only
  readthrough, stale reviewer output, or external review pass as equivalent to
  the packaged Codexy reviewer agent gate for the current diff and evidence.
