# Execution Budget

Non-trivial work MUST use finite budgets for implementation, repair, review, and
fanout. Churn and waiting MUST NOT renew those budgets.

Each non-trivial change MUST keep a finite requirement list, affected-check
list, and termination condition. Once the requirements, relevant checks, and
selected review when applicable are satisfied with no unresolved in-scope
defect, the work MUST finish. Additional tests, broad rechecks, or reviewers
require a named unmet criterion or concrete unresolved risk; an unspecified
desire for more confidence does not extend the work.

Evidence reuse is per check. A child MAY hand off preserved execution evidence
after a current applicability assessment, and the parent MAY consume it without
a duplicate default full-suite run. Relevant changes invalidate affected checks;
unknown impact, missing evidence, unobservable results, failures, and actual
findings remain unresolved states rather than reasons to silently pass or hide
the gap.

- Non-Sentinel fanout MUST be no more than three concurrent helpers.
- The normal current-head path uses one selected proportionate reviewer when the
  chosen profile requires one. `PASS`, `BLOCK`, and `UNOBSERVABLE` are actual
  results; `PENDING` and `RUNNING` remain observations of that same reviewer.
  This path has no fixed review-count, history, or disposition quota.
- An explicitly selected legacy history/transition path MAY use its recorded
  limits and disposition rules. Those fields MUST NOT be added to or required by
  a compact current-head state.
- Additional reviewers, broad rechecks, or semantic evaluators MUST be tied to
  an explicit requirement or a concrete unresolved risk; they MUST NOT be an
  automatic stack.
- For an external-only task, make one post-idle handoff and finish. Waiting or
  resource exhaustion MUST NOT create a blocked state when a safe default action
  exists.
