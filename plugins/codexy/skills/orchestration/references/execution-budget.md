# Finite execution budgets

Non-trivial lanes MUST set finite implementation, repair, and review caps.
Continuation MUST satisfy a criterion or remove a blocker; churn and waits do
not renew it. Parent fanout allows 3 non-Sentinel specialists.

At most 3 issue-wide terminal `PASS`, `BLOCK`, or `UNOBSERVABLE` verdicts;
`PENDING` and `RUNNING` do not count. A third `BLOCK` requires every
root repair and exact-head proof, without a fourth review. A third
`UNOBSERVABLE` needs maintainer disposition and equivalent proof. All test, CI,
thread, safety, ownership, LOC, and merge gates remain.

When only an external event remains, send one idle-wait handoff and finish.
Waiting or exhaustion MUST NOT block; only a material unanswered decision
without a safe default may.
