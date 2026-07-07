---
name: proof-driven-completion
description: MUST use before claiming work is done, opening or merging a PR, handing off, closing an issue, reporting success, or completing a goal for code, docs, workflow, UI, plugin, marketplace, or release tasks.
---

# Proof-Driven Completion

## Purpose

Completion is a claim about the current state, not a feeling about effort. This
skill requires evidence that directly matches every explicit requirement before
the agent says work is done, closes an issue, merges a PR, or marks a goal
complete.

## Completion Audit

1. Restate the requested outcome.
2. MUST list every explicit requirement, named file, command, review gate, external
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
4. MUST inspect the current authoritative source. MUST NOT rely on memory, intent, or
   earlier output unless it is explicitly marked as stale supporting context.
5. MUST classify each item as proved, contradicted, incomplete, too weak, or missing.
6. MUST continue work until every required item is proved, or report the exact
   blocker without calling the task complete.

## Required Checks

- MUST run `git diff --check` before pushing or opening a PR.
- MUST inspect `git status --short` and MUST NOT stage unrelated files.
- MUST parse structured files with an appropriate parser when possible.
- For Codexy plugin architecture changes, validate LSP config, MCP config,
  role metadata or custom agent TOMLs, and thread/worktree orchestration
  wording. MUST run `scripts/validate-plugin-config --check` when that
  script is present in the current revision.
- For code-touching or code-adjacent runtime changes, include Codexy
  `codegraph` MCP exploration evidence when the MCP is available, plus direct
  file-read confirmation before claiming the touched surface is understood.
- For non-trivial code, validator, harness, or workflow-rule changes, MUST run a
  touched implementation-file LOC gate before PR readiness or handoff:
  `scripts/validate-plugin-config --check-touched-loc --base-ref <base>`.
  MUST treat files over the 250 LOC target as failing evidence unless the tracked
  Codexy LOC exception mechanism names the file and rationale. Handoff or PR
  body prose alone is not proof of an exception.
- For plugin skills, MUST confirm every `SKILL.md` has valid YAML frontmatter with
  `name` and `description`.
- For GitHub PR work, MUST inspect PR state, latest head SHA, comments, reviews,
  review threads, and Codex connector output on the current head.
- When a handoff or final answer reports addressed review feedback, MUST include
  GraphQL `reviewThreads.nodes` in the PR state evidence and MUST run
  `scripts/validate-plugin-config --check-completion-handoff`; addressed
  unresolved threads, including outdated-but-fixed threads, MUST be resolved or
  covered by an accepted no-change rationale before readiness evidence is
  accepted.
- For child-owned PRs, MUST route actionable review feedback back to the owning
  child thread. The parent thread may coordinate, but it MUST NOT merge until
  the child thread returns current verification or a documented non-change
  rationale.
- Before accepting child handoffs that claim clean, synced, pushed, PR-ready,
  or parent-handoff-ready state, the parent MUST verify current `git status`,
  local head, remote ref, PR head, merge state, and unresolved review threads.
  The parent MUST NOT accept handoff prose when those current surfaces
  contradict it.
- If a child-owned PR handoff or final-answer evidence mentions parent-authored
  implementation or review-response commits, MUST run
  `scripts/validate-plugin-config --check-child-lane-ownership --evidence-file <path>`.
  A failing result blocks completion unless the evidence records explicit
  maintainer reassignment of implementation ownership to the parent.
- For delegated lanes that need their own branch, worktree, PR, or durable
  child context, MUST require evidence that the child was created, forked, or
  assigned before implementation patches began. If parent-authored draft edits
  exist, MUST require recovery evidence showing the parent stopped editing,
  disclosed the mistake, protected user and other-agent work, and handed the
  draft diff to the owning child thread.
- For completion, merge, or default Codexy merge-flow requests, MUST NOT treat a
  PR that remains open as completion unless the maintainer explicitly requested stop, wait,
  draft-only, no-merge, or leave-open behavior. When a final answer or handoff
  artifact may claim completion while the matching PR is open, MUST run
  `scripts/validate-plugin-config --check-completion-handoff --handoff-file <report> --pr-state-file <gh-pr-view-json>`
  against current PR state before accepting the claim.
- For every non-trivial atomic unit, MUST require evidence that the owning thread
  ran the packaged Codexy reviewer agent defined by
  `plugins/codexy/agents/codexy-sentinel.toml` before handoff, PR readiness,
  completion, or parent acceptance. The reviewer gate MUST cover the current
  diff, exact head or file state, lane scope, touched implementation-file LOC
  evidence, verification outputs, and evidence. Arbitrary reviewer agents,
  generic role names, parent-only
  readthroughs, stale reviewer output, or external review passes are not
  substitutes for this gate.
- MUST re-run verification after addressing review feedback.
- For delegated non-trivial or multi-step child implementation lanes, MUST verify
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
  Goal-tool evidence MUST name real Codex goal surfaces such as
  `create_goal`, `get_goal`, or `update_goal` when they are available.
  Prose-only `Goal:` or `Todo:` text is not evidence of real goal or todo/plan
  tool use.
  For an atomic trivial child lane, MUST require an explicit not-applicable rationale
  instead of silently skipping the execution discipline.

## Evidence Rules

- Evidence MUST be current for the file state, commit, PR head, runtime, or
  external setting being claimed.
- Narrow evidence proves only narrow claims. A parser check does not prove UX; a
  unit test does not prove GitHub settings; an `eyes` reaction does not prove
  Codex review completion.
- Eyes-only current-head `@codex review` evidence is not merge-ready. MUST require
  actual Codex review output, an explicit completion signal, or a maintainer
  override before reporting review completion or readiness to merge.
- If new commits land after review, request or wait for fresh review on the new
  head.
- If review feedback is addressed by a child thread, evidence MUST include the
  child thread result, the exact new head, and the rerun verification.
- If a fresh `@codex review` request for the current head already has `eyes`,
  MUST NOT send duplicate requests for the same head. MUST continue polling and
  waiting for review output. If it is unusually stale, document the status and
  MUST use a distinct escalation rationale instead of repeated blind requests.
- If a command was skipped, say so with the reason.
- If evidence is local and untracked, MUST summarize it or give the ignored evidence
  path; MUST NOT commit scratch artifacts unless requested.
- If a dependency PR has not yet landed, label validator, LSP, MCP, role
  metadata, custom agent TOML, thread, or worktree evidence as deferred instead
  of claiming completion.

## Final Report Shape

MUST include:

- outcome,
- changed files or surfaces,
- verification commands and results,
- external observations such as PR review state or UI behavior,
- not run,
- blockers or residual risks.

## Stop Conditions

- MUST stop and fix if proof contradicts the claim.
- MUST stop and ask only when the missing proof requires a secret, account action,
  destructive operation, or human-only decision.
- MUST NOT call `update_goal(status="complete")` until every requirement has
  current matching proof and no required work remains.
- MUST NOT call `update_goal(status="blocked")` merely because Codex connector
  review, child-thread work, queued worktree/thread setup, or asynchronous tool
  completion is pending. MUST continue polling, send follow-up prompts as needed,
  MUST route review feedback to the owning child thread, and MUST keep the goal active
  until a repeated true impasse prevents meaningful progress without user input
  or an external state change.
- MUST NOT accept a non-trivial child implementation handoff as complete when it
  omits actual goal-tool usage, actual todo/plan tool usage, required
  situational multi-agent usage, a concrete not-useful rationale tied to
  atomicity or tiny scope, or unavailable-tool fallback evidence required by
  the orchestrator assignment.
  Using only one of goal or todo/plan is insufficient unless the missing tool
  was unavailable and the child reported that unavailability with its fallback.
- MUST NOT accept a non-trivial atomic unit as complete when it omits the
  packaged Codexy reviewer agent result for the current diff, exact head or
  file state, lane scope, verification outputs, and evidence.

## Failure Modes

- Reporting a merge before verifying branch deletion and main sync.
- Reporting completion after opening a clean PR while merge gates are not
  intentionally deferred.
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
