# Review lifecycle

A reviewer MUST use the current diff and exact current head. `PASS`, `BLOCK`,
and `UNOBSERVABLE` are terminal results. `PENDING` and `RUNNING` are
non-terminal observations of the same active reviewer and MUST remain pending
until its actual result arrives.

## Current-head path

The default `codexy.review-control-state.v1` control is deliberately compact.
It binds the selected profile, the policy reviewer when applicable,
`reviewed_head`, one actual `terminal_result` or non-terminal `status`, and
`unresolved_findings`. It MUST match the authenticated current PR head and the
selected reviewer policy. It MUST NOT require a genesis record, prior snapshot,
transcript import, quota counter, invocation telemetry, or disposition ledger.

The normal sequence is:

1. capture the current PR state and exact head;
2. run the one proportionate reviewer selected by the profile when a reviewer
   is required;
3. keep the same reviewer and current-head binding while it is pending;
4. treat `PASS` with no unresolved findings as review evidence, while
   `BLOCK`, `UNOBSERVABLE`, a stale head, failed relevant check, or an actual
   unresolved finding remains blocking; and
5. fix an actual finding in the owning lane, rerun the relevant verification,
   and validate the new current head.

Missing historical evidence is an unknown limitation, not a synthetic result or a
new approval request. It MUST NOT block unrelated authorized preparation. A
current-head reviewer result does not imply that a connector review, semantic
evaluator, repeated broad check, or evidence artifact is needed. Those additions
require an explicit user or repository requirement or a concrete unresolved
risk.

The current PR snapshot is authoritative for repository, PR, base, and head
identity. A completion or readiness check MUST consume the current snapshot and
relevant checks; it MUST NOT reconstruct old review history merely because an
older record is absent. The owning child keeps implementation and
review-response ownership, and the parent keeps merge and publication
authority. The selected reviewer MUST NOT be interrupted, replaced, duplicated,
or turned into a second review because a wait is inconvenient.

## Explicit legacy review-state path

A control containing `full_review_count`, `delta_review_count`,
`terminal_review_count`, `terminal_review_limit`,
`terminal_review_history`, `pre_pr_import`, `native_history_recovery`,
`native_history_provenance`, `reviewer_migration`, `post_cap_re_review`, or
`final_disposition` opts into
the existing legacy path. Use it only when an explicit requirement or a concrete
unresolved risk needs historical reconstruction or a bounded disposition. The
legacy validators MUST preserve the actual reviewer tuples, reviewed heads,
findings, source provenance, and authenticated current/previous PR snapshots;
`previous_control_state` remains rejected. Missing or incomplete source
evidence stays unknown and MUST NOT be replaced with a fabricated verdict.

Pre-PR import and native recovery inputs remain non-admitted until a real
current-head review is available. They MUST preserve the source-owned records
and MUST NOT authorize readiness, completion, merge, or another review by
themselves. See [native review history](native-review-history.md).

A legacy transition may use its recorded full, delta, or post-cap rules only
within that selected path. It MUST preserve an existing history prefix, reject
duplicate, reordered, truncated, or fabricated events, keep real findings
blocking, and continue to enforce code, relevant checks, ownership, safety,
review-thread, LOC, and merge gates. A third `BLOCK` or
`UNOBSERVABLE` may use the existing bounded disposition only when its
authenticated source contract is satisfied; it MUST NOT create a synthetic
`PASS` or a fourth reviewer. See [review profiles](review-profiles.md) and
[authenticated finding-disposition CI](finding-disposition-ci.md).

Headings, prose, optional receipts, and omitted legacy fields MUST NOT override
direct current-head facts. The executable validator remains the authority for
both the compact current-head and explicitly selected legacy paths.
