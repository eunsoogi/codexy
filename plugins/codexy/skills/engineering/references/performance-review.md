# Performance review

## Method

Use this reference only when the request explicitly asks for performance cost or
test efficiency. Treat it as an evidence review, not a blanket test reduction
target.

1. Choose one representative workload and record its exact repository head,
   environment, input shape, and cold or warm state. Reuse existing metrics and
   instrumentation before adding measurement code.
2. Use the same invocation and state for the comparison. Record elapsed time,
   CPU, peak RSS, disk usage, child-process count and time, and fixture bytes.
   Include setup or compile cost when it is part of the requested path.
3. Record the unit, sample or aggregation, source, and comparison baseline for
   every metric. For an uncollected metric, write exactly "not measured". Do not
   turn its absence into zero, savings, or success.
4. Call a value an estimate only when it is derived from disclosed inputs and a
   reproducible method. Record that method and its evidence beside the estimate.

## Test removal or merge

For every candidate test to delete or merge, preserve this evidence chain:

`requirement protected → replacement oracle → unique regression case retained`

- Requirement protected names the behavior, compatibility, or security
  invariant.
- Replacement oracle names the independent observation that detects a violation;
  it MUST NOT only restate implementation form.
- Unique regression case retained identifies the remaining case that fails when
  that requirement regresses and explains why equivalent coverage does not
  already retain it.

Deleting a test and seeing a green suite is not a replacement oracle; it only
shows that a check disappeared. Keep actual regression and security checks, and
compare fixture and child-process cost on the same workload and head. If there
is no measured improvement, or meaning preservation fails, report the cause and
smallest follow-up without removing tests or lowering the bar.

## Decision and handoff

Do not assume a particular test runner. Name the authentic command and record
the workload, head, state, measurements, unavailable metrics, and cleanup. A
timeout, skipped measurement, cache change, retry, sleep, or sharding change
alone is not a measured improvement.

End the review with these four items, in order:

- Problem: what cost or redundant coverage was observed.
- Evidence: the current measurements and preservation chain.
- Smallest improvement: the narrowest justified change.
- Verification: the authentic regression, security, and cost checks that ran.

Do not add a separate security checklist; retain the existing security and
regression checks relevant to the workload.
