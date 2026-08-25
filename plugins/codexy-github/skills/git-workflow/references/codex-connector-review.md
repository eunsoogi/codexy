# Optional Manual Codex Connector Review

## Applicability

This procedure applies only when the Codex connector is available to the
parent/orchestrator and active repository policy requires one explicit manual
connector review. Otherwise, it is not a merge gate and an agent MUST NOT
invent, emulate, or wait for `@codex review` evidence. When applicable, Codex
connector automatic review MUST remain disabled.

## Procedure When Applicable

1. [automatic-disabled] Codex connector automatic review MUST remain disabled.
2. [proof-ci-before-review] Before requesting review, parent/orchestrator MUST
   complete local affected proof and wait for required CI readiness on the
   frozen exact head.
3. [exactly-one-review] After local proof and required CI readiness,
   parent/orchestrator MUST request exactly one `@codex review` before merge
   after an owning child returns the packaged multi-agent review policy's
   merge-eligible disposition on the frozen exact head.
4. [wait-batch] Parent/orchestrator MUST wait for the requested review's
   terminal output and batch every actionable connector finding into one repair
   cycle.
5. [child-repair-profile] Owning child MUST repair the batch. When a terminal
   profile-review slot remains, it MUST run only the permitted same-profile
   delta recheck on the repaired exact head. When the issue-wide terminal
   profile-review quota is exhausted, it MUST instead record the typed post-cap
   connector-repair disposition against the authentic delta head, keep the
   repaired current head non-ready, repair every in-scope connector finding, and
   produce current exact-head proof without fabricating `PASS`, `APPROVED`, or
   `PARENT_DECISION`, requesting a fourth profile review, or requesting another
   connector review.
6. [no-automatic-or-duplicate] Automatic, per-push, duplicate, unchanged-head,
   and piecemeal Codex connector review requests MUST NOT be made.
7. [material-expansion-exception] Another connector review MUST NOT be requested
   unless a maintainer explicitly authorizes it after material scope expansion.

Human review, selected-profile review evidence, title, label,
completion-handoff, CI, review-thread resolution, and merge-message gates MUST
remain required. Existing actionable human or connector review threads MUST
remain merge-blocking until the current head resolves them or an accepted
no-change rationale covers them.

## Ownership

Only the parent/orchestrator requests the one connector review and waits for its
terminal output. The owning child receives the batched actionable findings, owns
any repair and the quota-permitted delta recheck or typed post-cap
connector-repair disposition, and MUST NOT request a connector review. This
procedure does not enable automatic connector review or replace any existing
human, selected-profile, title, label, completion-handoff, CI, or merge-message
gate.
