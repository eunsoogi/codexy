# Diagnosis

## Method

MUST find the cause before applying a fix.

1. Reproduce and preserve the smallest faithful failure, recording inputs,
   expected/actual output, environment, version, and revision when relevant.
2. Form evidence-based hypotheses; test one variable at a time with suitable
   logs, assertions, LSP, history, or process observation.
3. Isolate the root cause, apply the smallest explanatory repair, and rerun the
   reproduction plus regression proof.

## Constraints

- MUST NOT fix before reproduction or call a failure flaky without proof.
- The regression must fail before the repair, or justify why it cannot.
- Verify user-visible failures on their external surface and remove temporary
  instrumentation.
