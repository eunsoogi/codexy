# Diagnosis

## Method

MUST find the cause before applying a fix.

1. Reproduce the smallest faithful failure; record inputs, expected/actual,
   environment, version, and relevant revision.
2. Form hypotheses from evidence; test one variable at a time with
   logs, assertions, LSP, history, or process observation.
3. Isolate root cause, apply the smallest explanatory repair, and rerun the
   reproduction plus regression proof.

## Constraints

- MUST reproduce before fixing and MUST NOT call a failure flaky without proof.
  If reproduction is impossible, MUST record why and limit progress to
  evidence-bounded work.
- The regression MUST fail before repair, or MUST justify why it cannot.
- Verify user-visible failures on their external surface and remove temporary
  instrumentation.
