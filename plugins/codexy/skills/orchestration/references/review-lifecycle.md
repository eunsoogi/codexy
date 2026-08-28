# Selected review lifecycle

MUST use exactly the reviewer in `review-profiles.json` on the current diff and
head. Its terminal verdict is `PASS`, `BLOCK`, or `UNOBSERVABLE`; only `PASS`
satisfies readiness before the issue-wide three-verdict cap is exhausted.

Retain and observe a live reviewer read-only. MUST NOT message, interrupt,
replace, duplicate, or poll it. `PENDING` and `RUNNING` are non-terminal. Repair
every in-scope finding and refresh exact-head proof. A fourth selected review is
forbidden; cap exhaustion waives no test, CI, thread, safety, ownership, LOC,
connector, or merge gate.
