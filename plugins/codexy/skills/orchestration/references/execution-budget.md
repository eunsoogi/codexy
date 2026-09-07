# Execution Budget

Non-trivial work MUST use finite budgets for implementation, repair, review, and
fanout. Churn and waiting MUST NOT renew those budgets.

- Non-Sentinel fanout MUST be no more than three concurrent helpers.
- The issue-wide terminal review limit is three. `PASS`, `BLOCK`, and
  `UNOBSERVABLE` consume that limit; `PENDING` and `RUNNING` are non-terminal
  observations and do not.
- At the limit, use a typed final disposition with current proof. A fourth
  review MUST NOT occur, and the remaining test, validator, CI, ownership,
  safety, LOC, and merge gates still apply.
- For an external-only task, make one post-idle handoff and finish. Waiting or
  resource exhaustion MUST NOT create a blocked state when a safe default action
  exists.
