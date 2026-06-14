---
name: debugging
description: Use when behavior is wrong, tests fail, processes hang, output is unexpected, regressions appear, UI breaks, GitHub automation misbehaves, or a root cause is unknown.
---

# Debugging

## Purpose

Find the cause before applying the fix. Debugging is reproduction, evidence
collection, hypothesis testing, minimal repair, and regression proof.

## Workflow

1. Reproduce the symptom with the smallest faithful command or user action.
2. Capture:
   - exact inputs,
   - actual output,
   - expected output,
   - versions,
   - environment details,
   - timestamp or commit when relevant.
3. Preserve the failing proof. Do not edit production files until the failure is
   reproducible or the blocker is recorded.
4. Generate hypotheses from evidence, not guesses.
5. Test one hypothesis at a time:
   - logs or traces around suspected boundary,
   - assertions around invariants,
   - LSP/typecheck/static analysis for code-shape failures,
   - blame, bisect, or diff comparison for regressions,
   - network, process, or filesystem observation for integration failures.
6. Apply the smallest fix that explains all observed evidence.
7. Re-run the original reproduction, targeted regression proof, and broader
   verification sized to blast radius.
8. Remove temporary instrumentation, debug logs, local-only flags, and scratch
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

- Do not fix before reproducing unless reproduction is impossible and the
  blocker is explicitly recorded.
- Do not call a test flaky before proving the failure mode.
- Do not hide the symptom with retries, sleeps, broad catches, or skipped tests.
- Do not leave instrumentation in the final diff unless it is intentional
  product behavior.

## Evidence Rules

- The final fix must explain every observed symptom.
- The original reproduction must pass after the fix.
- A regression proof should fail on the old behavior or be justified when that
  is impossible.
- If the bug is externally observable, verify the external surface after the
  local fix.

## Failure Modes

- Debugging from intuition instead of reproduction.
- Changing multiple variables in one experiment.
- Stopping after the first green targeted check while broader affected surfaces
  remain untested.
- Keeping local paths, machine-specific assumptions, or temporary logs in the
  repository.
