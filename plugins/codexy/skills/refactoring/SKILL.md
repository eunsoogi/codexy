---
name: refactoring
description: MUST use when restructuring existing code without changing behavior, splitting large files or modules, reducing coupling, extracting helpers, simplifying boundaries, or keeping implementation files at or below the default 250 LOC target.
---

# Refactoring

## Purpose

Improve code shape while preserving behavior. MUST use this skill for scoped
refactors, large-file splits, module boundary cleanup, helper extraction,
dependency inversion, naming cleanup, and review-driven maintainability work.

## Default LOC Target

- MUST keep code files at or below 250 lines of code by default.
- MUST treat the 250 LOC target as a design pressure, not permission for churn.
- Before PR readiness or handoff, MUST run
  `scripts/validate-plugin-config --check-touched-loc --base-ref <base>` over
  the current branch and include the command output in evidence.
- If a code or test-harness file MUST exceed 250 LOC, MUST add or reference the
  tracked Codexy LOC exception entry with a narrow rationale. MUST NOT rely on
  PR body prose alone, and MUST NOT hide the exception.
- MUST NOT split files mechanically when the result obscures public contracts,
  makes navigation worse, or creates circular dependencies.

## Workflow

1. MUST read the issue, current diff, owning `AGENTS.md`, and relevant project
   skills before editing.
2. MUST inspect callers, exports, tests, fixtures, and runtime entry points for the
   code being moved or split.
3. MUST establish behavior-preserving proof:
   - MUST run existing focused tests when they exist,
   - MUST add or keep regression tests when behavior risk exists,
   - MUST capture CLI, harness, or UI evidence when the changed surface is external.
4. MUST identify the smallest coherent refactor:
   - MUST extract one helper or module boundary,
   - MUST split one large file by stable responsibility,
   - MUST remove one duplication cluster,
   - MUST isolate one dependency direction.
5. MUST move code while preserving public contracts. MUST keep exported names, CLI flags,
   serialized formats, API shapes, and plugin manifests stable unless the issue
   explicitly authorizes a contract change.
6. MUST re-run focused verification after every meaningful move. Broaden checks when
   shared code, plugin loading, harness execution, or generated artifacts are
   affected.
7. MUST report changed files, remaining large-file exceptions, verification evidence,
   and any follow-up refactors that become separate issues.

## Guardrails

- MUST NOT mix feature work, bug fixes, formatting sweeps, or unrelated cleanup
  into a refactor PR.
- MUST NOT weaken, delete, skip, or rewrite tests just to make a refactor pass.
- MUST NOT change behavior silently. If a behavior change is discovered, MUST stop and
  split it into an explicit fix or feature lane.
- MUST NOT move code before reading its callers and tests.
- MUST NOT rely on green tests alone when the user-visible surface is a CLI,
  plugin install, GitHub workflow, browser page, or desktop app.
- MUST NOT refactor across unrelated bounded contexts in one branch.

## Splitting Large Files

When a file exceeds the 250 LOC target:

1. MUST classify responsibilities: parsing, validation, orchestration, IO, rendering,
   adapters, domain rules, and tests.
2. MUST choose seams that preserve existing imports or allow a small compatibility
   re-export.
3. MUST extract pure helpers before stateful orchestration when possible.
4. MUST keep side effects near entry points and move deterministic logic behind
   named functions.
5. MUST preserve module-level comments, public docstrings, and error messages unless
   the refactor intentionally improves them.
6. MUST re-run tests or harness commands that exercise both the old entry point and
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
