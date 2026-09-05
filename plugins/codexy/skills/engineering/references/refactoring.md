# Refactoring

## Method

MUST improve structure while preserving behavior and public contracts.

1. Inspect callers, exports, tests, fixtures, runtime entries, and diff before
   moving code or prose.
2. Establish focused behavior-preserving proof and an authentic surface check
   when the changed contract is externally observable.
3. Choose one coherent seam: helper or module extraction, stable responsibility
   split, duplicate removal, test-target split, or dependency isolation.
4. Preserve exported names, flags, formats, APIs, manifests, side-effect
   boundaries, comments, and error text unless the issue explicitly changes
   them.
5. Re-run proof after each meaningful move and describe the structural boundary
   or duplication removed.

MUST NOT weaken, delete, skip, or rewrite tests just to pass a refactor.

## Governed LOC contract

- The canonical policy is [governed-code.md](governed-code.md): every governed
  file MUST stay at or below 250 physical lines with no exception.
- Before handoff, run the package checker with one explicit `--path` per
  applicable touched file; it MUST resolve package policy without assuming a
  checkout root or traversing directories.
- A 251-line governed file blocks readiness. Blank-line deletion or collapsed
  readable multiline code, tests, or instructions is formatting-only and MUST
  NOT prove remediation.
- Split by stable responsibility without obscuring public contracts, worsening
  navigation, creating circular dependencies, or mixing feature work and
  unrelated cleanup.

## Performance and test-preservation review

Continue to preserve behavior and public contracts when a refactor changes test
targets, fixtures, or measured execution paths. Use
[Performance review](performance-review.md) only when the request explicitly
asks for cost or test efficiency. Before deleting or merging a test, preserve
this evidence chain:

`requirement protected → replacement oracle → unique regression case retained`

A green suite after deleting a test proves only that the deleted check no longer
runs. Keep the actual regression and security checks, and record missing
measurements as `not measured`.

The 250-line limit and readable structural-remediation rules above remain
unchanged. End the result with the problem, evidence, smallest improvement, and
verification. Do not add a separate security checklist.
