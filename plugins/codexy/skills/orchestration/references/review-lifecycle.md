# Profile-Selected Multi-Agent Review Lifecycle

MUST select exactly the reviewer in `review-profiles.json`. Its terminal result
is `PASS`, `BLOCK`, or `UNOBSERVABLE`; a live reviewer is retained and observed
read-only.

## Terminal Review Cap

The lane MUST carry one issue-wide count of terminal profile-selected verdicts
across goals, repair stages, compaction, reauthorization, and route resets.
`PENDING` and `RUNNING` do not count. The count MUST NOT exceed three.

Before all three terminal verdicts are consumed, only `PASS` satisfies the
profile-selected multi-agent review gate. A third `PASS` proceeds normally.

After all three terminal verdicts are consumed, a third-`BLOCK` final repair or
third-`UNOBSERVABLE` maintainer disposition satisfies that gate and permits the
completion procedure without fabricating `PASS` or requesting a fourth
profile-selected reviewer. A third-`BLOCK` final repair MUST repair every
in-scope root finding and have current exact-head proof. A third `UNOBSERVABLE`
disposition MUST have equivalent maintainer-owned current proof.

## Repairs And Remaining Gates

The owner MUST continue repairing every in-scope review finding and every
in-scope test, validator, or CI failure until current proof is green. A later
repair MUST NOT request a fourth profile-selected reviewer. Review-cap
exhaustion MUST NOT block a goal: complete the finite phase, use the idle-wait
handoff when appropriate, and start a fresh goal only for a later authorized
phase.

The cap waives only a fourth profile-selected review. Tests, validators,
exact-head CI, actionable human or connector threads, ownership, safety, LOC,
and merge gates remain mandatory. A connector review is outside this counter and
MUST NOT reopen an exhausted profile-review loop.
