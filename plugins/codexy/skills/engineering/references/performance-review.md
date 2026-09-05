# Performance review

## Method

MUST use this reference only when the request explicitly asks for performance
cost or test efficiency. MUST treat it as an evidence review, not a blanket test
reduction target.

1. MUST choose one representative workload and MUST record its exact repository
   head, environment, input shape, and cold or warm state. MUST reuse existing
   metrics and instrumentation before adding measurement code.
2. MUST use the same invocation and state for the comparison. MUST record
   elapsed time, CPU, peak RSS, disk usage, child-process count and time, and
   fixture bytes. MUST include setup or compile cost when it is part of the
   requested path.
3. MUST record the unit, sample or aggregation, source, and comparison baseline
   for every metric. MUST record an uncollected metric as "not measured". MUST
   NOT turn its absence into zero, savings, or success.
4. MUST call a value an estimate only when it is derived from disclosed inputs
   and a reproducible method. MUST record that method and its evidence beside
   the estimate.

## Test removal or merge

For every candidate test to delete or merge, MUST preserve this evidence chain:

`requirement protected → replacement oracle → unique regression case retained`

- MUST name the requirement protected as the behavior, compatibility, or
  security invariant.
- MUST name the replacement oracle as the independent observation that detects a
  violation; it MUST NOT only restate implementation form.
- MUST identify the unique regression case retained that fails when that
  requirement regresses and MUST explain why equivalent coverage does not
  already retain it.

MUST NOT treat deleting a test and seeing a green suite as a replacement oracle;
it only shows that a check disappeared. MUST keep actual regression and security
checks, and MUST compare fixture and child-process cost on the same workload and
head. If there is no measured improvement, or meaning preservation fails, MUST
report the cause and smallest follow-up without removing tests or lowering the
bar.

## Decision and handoff

MUST NOT assume a particular test runner. MUST name the authentic command and
MUST record the workload, head, state, measurements, unavailable metrics, and
cleanup. MUST NOT treat a timeout, skipped measurement, cache change, retry,
sleep, or sharding change alone as a measured improvement.

MUST end the review with these four items, in order:

- Problem: MUST state what cost or redundant coverage was observed.
- Evidence: MUST state the current measurements and preservation chain.
- Smallest improvement: MUST state the narrowest justified change.
- Verification: MUST state the authentic regression, security, and cost checks
  that ran.

MUST NOT add a separate security checklist; MUST retain the existing security
and regression checks relevant to the workload.
