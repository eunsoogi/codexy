# Review lifecycle

This reference owns reviewer-state validity and current-head binding. A reviewer
MUST use the current diff and exact current head. `PASS`, `BLOCK`, and
`UNOBSERVABLE` are terminal results. `PENDING` and `RUNNING` are non-terminal
observations of the same active reviewer and MUST remain pending until its
actual result arrives.

## Current-head path

The current review control is deliberately compact. It binds the explicit
`review_requested` decision, Sentinel when selected, `reviewed_head`, one actual
`terminal_result` or non-terminal `status`, and `unresolved_findings`. It MUST
match the authenticated current PR head and the selected review decision. It
MUST NOT require a genesis record, prior snapshot, transcript import, quota
counter, invocation telemetry, or disposition ledger. The compact producer does
not require `issue_number` or a prior control state; when supplied,
`previous_control_state` MUST be rejected. Absence of a prior control remains
valid.

Legacy v1 review records remain historical and are validated separately. MUST
NOT rewrite their profile or reviewer fields to resemble current v2 records, or
infer a current review requirement from a historical profile. Current v2 records
use optional review rather than profile tiers or a review quota.

The normal sequence is:

1. Capture the current PR state and exact head.
2. Decide whether to use [optional final review](review-profiles.md). When
   selected, run Sentinel only after implementation and applicable checks. When
   not selected, record no review rather than fabricating a reviewer PASS.
3. Keep that reviewer and its current-head binding while it is pending.
4. Treat `PASS` with no unresolved findings as review evidence. `BLOCK`,
   `UNOBSERVABLE`, a stale head, or an actual unresolved finding is not a
   current-head pass.
5. Fix an actual finding in the owning lane, run affected checks selected under
   [quality assurance](../../engineering/references/quality-assurance.md), and
   have the selected reviewer assess the new current head within the
   instruction-only three-assessment budget. A terminal result for the old head
   remains historical; exhausted review guidance MUST NOT erase findings.

## Verification and outcome boundaries

Check selection, proof depth, and test-oracle preservation belong to
[quality assurance](../../engineering/references/quality-assurance.md). The
validity or reuse of execution evidence and overall outcome judgment belong to
[proof-driven completion](../../proof-driven-completion/SKILL.md). A reviewer
result is one input to that outcome; it MUST NOT stand in for the full
requirement and evidence assessment. Additional checks or reviews follow the
[execution budget](execution-budget.md) and require an unmet criterion or
concrete unresolved risk.

Missing historical transcript, import, invocation, or quota data MUST NOT be
converted into a synthetic reviewer result or a new approval request. The
compact current-head path does not require those records. A missing history
record does not make an otherwise valid current-head result stale.

The current PR snapshot is authoritative for repository, PR, base, and head
identity. This lifecycle MUST NOT reconstruct old review history merely because
an older record is absent. The owning child keeps implementation and
review-response ownership, and the parent keeps merge and publication authority.
The selected reviewer MUST NOT be interrupted, replaced, duplicated, or turned
into a second review because a wait is inconvenient.
