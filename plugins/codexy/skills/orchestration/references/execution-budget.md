# Finite execution budgets

Non-trivial lanes MUST set finite implementation, repair, and review caps.
Continuation MUST satisfy a criterion or remove a blocker; churn and waits do
not renew it. Parent fanout allows 3 non-Sentinel specialists.

Terminal `PASS`, `BLOCK`, or `UNOBSERVABLE` verdicts are limited to three per
issue; `PENDING` and `RUNNING` do not count. After a third `BLOCK`, repair root
findings and refresh exact-head proof without a fourth review. A third
`UNOBSERVABLE` needs maintainer disposition and equivalent proof. Test, CI,
thread, safety, ownership, LOC, and merge gates remain active.

When only an external event remains, send one idle-wait handoff and finish the
goal. Exhaustion or waiting MUST NOT mark it blocked; only a material unanswered
decision without a safe default may do so.
