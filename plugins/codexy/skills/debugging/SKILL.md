---
name: debugging
description: MUST use when behavior is wrong, tests fail, processes hang, output is unexpected, regressions appear, UI breaks, GitHub automation misbehaves, or a root cause is unknown.
---

# Debugging

## Purpose

MUST find the cause before applying the fix. Debugging is reproduction, evidence
collection, hypothesis testing, minimal repair, and regression proof.

When replying in Korean, MUST follow [Natural Korean User Replies](../codex-orchestration/references/natural-korean-responses.md) for the user summary while preserving exact debugging evidence separately.

## Workflow

1. MUST reproduce the symptom with the smallest faithful command or user action.
2. MUST capture:
   - exact inputs,
   - actual output,
   - expected output,
   - versions,
   - environment details,
   - timestamp or commit when relevant.
3. MUST preserve the failing proof. MUST NOT edit production files until the failure is
   reproducible or the blocker is recorded.
4. MUST generate hypotheses from evidence, not guesses.
5. MUST test one hypothesis at a time:
   - logs or traces around suspected boundary,
   - assertions around invariants,
   - LSP/typecheck/static analysis for code-shape failures,
   - blame, bisect, or diff comparison for regressions,
   - network, process, or filesystem observation for integration failures.
6. MUST apply the smallest fix that explains all observed evidence.
7. MUST re-run the original reproduction, targeted regression proof, and broader
   verification sized to blast radius.
8. MUST remove temporary instrumentation, debug logs, local-only flags, and scratch
   artifacts.

## Required Output

```text
Symptom:
Reproduction:
Expected:
Actual:
Hypotheses:
Experiment:
Result:
Fix:
Regression proof:
Cleanup:
```

## Gates

- MUST NOT fix before reproducing unless reproduction is impossible and the
  blocker is explicitly recorded.
- MUST NOT call a test flaky before proving the failure mode.
- MUST NOT hide the symptom with retries, sleeps, broad catches, or skipped tests.
- MUST NOT leave instrumentation in the final diff unless it is intentional
  product behavior.

## Evidence Rules

- The final fix MUST explain every observed symptom.
- The original reproduction MUST pass after the fix.
- A regression proof MUST fail on the old behavior or be justified when that
  is impossible.
- If the bug is externally observable, MUST verify the external surface after the
  local fix.

## Failure Modes

- Debugging from intuition instead of reproduction.
- Changing multiple variables in one experiment.
- Stopping after the first green targeted check while broader affected surfaces
  remain untested.
- Keeping local paths, machine-specific assumptions, or temporary logs in the
  repository.
