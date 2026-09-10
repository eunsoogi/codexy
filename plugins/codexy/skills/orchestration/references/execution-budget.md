# Execution Budget

Non-trivial work MUST use finite budgets for implementation, repair, review, and
fanout. Churn and waiting MUST NOT renew those budgets.

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
