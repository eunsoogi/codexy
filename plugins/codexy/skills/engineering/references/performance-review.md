# Performance review

## Method

MUST use this reference only when the request explicitly asks for performance
cost or test efficiency. MUST treat it as an evidence review, not a blanket test
reduction target.

First classify the requested outcome:

- Maintenance-only test cleanup removes or merges a duplicate check. It MUST
  satisfy the canonical
  [test-oracle preservation rule](quality-assurance.md#test-oracle-preservation);
  cost evidence MUST NOT override it, and a lower test count MUST NOT be claimed
  as a runtime improvement. CPU, RSS, disk, process, and similar profiling MUST
  NOT be required solely to justify an independently proven duplicate removal.
  MUST reuse existing metrics when available and record an unavailable value as
  `not measured`.
- A runtime performance optimization makes a cost or speed claim. It MUST use
  the comparable workload and cost evidence below, including setup or compile
  cost when that cost belongs to the requested path.

## Comparable cost evidence

1. MUST choose one representative workload and MUST record the baseline and
   candidate repository revisions separately, along with the environment, input
   shape, and cold or warm state. MUST reuse existing metrics and
   instrumentation before adding measurement code.
2. MUST keep the workload, invocation, environment, and cache state constant for
   the comparison. MUST record elapsed time, CPU, peak RSS, disk usage,
   child-process count and time, and fixture bytes.
3. MUST record the unit, sample or aggregation, source, and comparison baseline
   for every metric. MUST record an uncollected metric as "not measured". MUST
   NOT turn its absence into zero, savings, or success.
4. MUST call a value an estimate only when it is derived from disclosed inputs
   and a reproducible method. MUST record that method and its evidence beside
   the estimate.

## Decision and handoff

MUST NOT assume a particular test runner. For a runtime optimization, MUST name
the authentic command and record the workload, baseline and candidate revisions,
invocation, environment, cache state, measurements, unavailable metrics, and
cleanup. For maintenance-only cleanup, name the authentic command, reference the
QA oracle-preservation decision, record reused metrics or `not measured` values,
and describe cleanup without inventing a performance claim. MUST NOT treat a
timeout, skipped measurement, cache change, retry, sleep, or sharding change
alone as a measured improvement.

MUST end the review with these four items, in order:

- Problem: MUST state what cost or redundant coverage was observed.
- Evidence: MUST state the current measurements and QA preservation decision.
- Smallest improvement: MUST state the narrowest justified change.
- Verification: MUST state the authentic regression, security, and cost checks
  that ran.

MUST NOT add a separate security checklist; MUST retain the existing security
and regression checks relevant to the workload.
