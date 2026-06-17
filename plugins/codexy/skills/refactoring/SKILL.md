---
name: refactoring
description: Use when restructuring existing code without changing behavior, splitting large files or modules, reducing coupling, extracting helpers, simplifying boundaries, or keeping implementation files at or below the default 250 LOC target.
---

# Refactoring

## Purpose

Improve code shape while preserving behavior. Use this skill for scoped
refactors, large-file splits, module boundary cleanup, helper extraction,
dependency inversion, naming cleanup, and review-driven maintainability work.

## Default LOC Target

- Keep code files at or below 250 lines of code by default.
- Treat the 250 LOC target as a design pressure, not permission for churn.
- Before PR readiness or handoff, run
  `scripts/validate-plugin-config --check-touched-loc --base-ref <base>` over
  the current branch and include the command output in evidence.
- If a code or test-harness file must exceed 250 LOC, add or reference the
  tracked Codexy LOC exception entry with a narrow rationale. Do not rely on
  PR body prose alone, and do not hide the exception.
- Do not split files mechanically when the result obscures public contracts,
  makes navigation worse, or creates circular dependencies.

## Workflow

1. Read the issue, current diff, owning `AGENTS.md`, and relevant project
   skills before editing.
2. Inspect callers, exports, tests, fixtures, and runtime entry points for the
   code being moved or split.
3. Establish behavior-preserving proof:
   - run existing focused tests when they exist,
   - add or keep regression tests when behavior risk exists,
   - capture CLI, harness, or UI evidence when the changed surface is external.
4. Identify the smallest coherent refactor:
   - extract one helper or module boundary,
   - split one large file by stable responsibility,
   - remove one duplication cluster,
   - isolate one dependency direction.
5. Move code while preserving public contracts. Keep exported names, CLI flags,
   serialized formats, API shapes, and plugin manifests stable unless the issue
   explicitly authorizes a contract change.
6. Re-run focused verification after every meaningful move. Broaden checks when
   shared code, plugin loading, harness execution, or generated artifacts are
   affected.
7. Report changed files, remaining large-file exceptions, verification evidence,
   and any follow-up refactors that should become separate issues.

## Guardrails

- Do not mix feature work, bug fixes, formatting sweeps, or unrelated cleanup
  into a refactor PR.
- Do not weaken, delete, skip, or rewrite tests just to make a refactor pass.
- Do not change behavior silently. If a behavior change is discovered, stop and
  split it into an explicit fix or feature lane.
- Do not move code before reading its callers and tests.
- Do not rely on green tests alone when the user-visible surface is a CLI,
  plugin install, GitHub workflow, browser page, or desktop app.
- Do not refactor across unrelated bounded contexts in one branch.

## Splitting Large Files

When a file exceeds the 250 LOC target:

1. Classify responsibilities: parsing, validation, orchestration, IO, rendering,
   adapters, domain rules, and tests.
2. Choose seams that preserve existing imports or allow a small compatibility
   re-export.
3. Extract pure helpers before stateful orchestration when possible.
4. Keep side effects near entry points and move deterministic logic behind
   named functions.
5. Preserve module-level comments, public docstrings, and error messages unless
   the refactor intentionally improves them.
6. Re-run tests or harness commands that exercise both the old entry point and
   the extracted module.

## Required Handoff

```text
Refactor goal:
Behavior preserved:
Touched implementation LOC:
Files over 250 LOC:
Exceptions and rationale:
Public contracts checked:
Tests or regression proof:
Verification:
Follow-up issues:
```
