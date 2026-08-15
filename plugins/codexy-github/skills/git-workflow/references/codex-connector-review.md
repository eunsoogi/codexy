# Manual Codex Connector Review

Codex connector automatic review MUST remain disabled.

## Required Procedure

1. [automatic-disabled] Codex connector automatic review MUST remain disabled.
2. [proof-ci-before-review] Before requesting review, parent/orchestrator MUST complete local
   affected proof and wait for required CI readiness on the frozen exact head.
3. [exactly-one-review] After local proof and required CI readiness, parent/orchestrator MUST
   request exactly one `@codex review` after an owning child profile-selected reviewer PASS on a
   frozen exact head and before merge.
4. [wait-batch] Parent/orchestrator MUST wait for the requested review's terminal output and batch
   every actionable connector finding into one repair cycle.
5. [child-repair-profile] Owning child MUST repair the batch and run only the permitted same-profile
   delta recheck on the repaired exact head without requesting another connector review.
6. [no-automatic-or-duplicate] Automatic, per-push, duplicate, unchanged-head, and piecemeal Codex
   connector review requests MUST NOT be made.
7. [material-expansion-exception] Another connector review MUST NOT be requested unless a maintainer
   explicitly authorizes it after material scope expansion.

Human review, selected-profile review evidence, title, label, completion-handoff, CI, review-thread
resolution, and merge-message gates MUST remain required. Existing actionable human or connector
review threads MUST remain merge-blocking until the current head resolves them or an accepted
no-change rationale covers them.

## Ownership

Only the parent/orchestrator requests the one connector review and waits for its terminal output.
The owning child receives the batched actionable findings, owns any repair and permitted
same-profile delta recheck, and MUST NOT request a connector review. This procedure does not enable
automatic connector review or replace any existing human, selected-profile, title, label,
completion-handoff, CI, or merge-message gate.
