# Review lifecycle

A reviewer MUST use the current diff and exact current head. `PASS`, `BLOCK`,
and `UNOBSERVABLE` are terminal results. `PENDING` and `RUNNING` are
non-terminal observations of the same active reviewer and MUST remain pending
until its actual result arrives.

## Current-head path

The default `codexy.review-control-state.v1` control is deliberately compact. It
binds the selected profile, the policy reviewer when applicable,
`reviewed_head`, one actual `terminal_result` or non-terminal `status`, and
`unresolved_findings`. It MUST match the authenticated current PR head and the
selected reviewer policy. It MUST NOT require a genesis record, prior snapshot,
transcript import, quota counter, invocation telemetry, or disposition ledger.

The normal sequence is:

1. capture the current PR state and exact head;
2. run the one proportionate reviewer selected by the profile when a reviewer is
   required;
3. keep the same reviewer and current-head binding while it is pending;
4. treat `PASS` with no unresolved findings as review evidence, while `BLOCK`,
   `UNOBSERVABLE`, a stale head, failed relevant check, or an actual unresolved
   finding remains blocking; and
5. fix an actual finding in the owning lane, rerun the relevant verification,
   and validate the new current head.

Verification consumption follows a separate per-check decision. For each result
that may be reused, preserve the original invocation revision, environment,
exact command, result, and execution state, then read back the current diff and
the relevant implementation, dependency/lock/configuration, fixtures, generated
inputs, and environment. A current applicability assessment MAY preserve a valid
passed result for an unchanged boundary or unrelated prose-only or other
metadata-only change; it MUST NOT rewrite the execution record or call it a
rerun. A relevant source, dependency, lock/configuration, shared fixture,
generated-input, or environment change, previous failure, unresolved risk, or
uncertain impact requires the affected checks again. Known integration changes
invalidate only affected evidence; uncertain integration impact warrants broader
checks.

The owning child MAY provide this preserved evidence for the parent to consume.
The parent MUST NOT duplicate the full suite by default solely to repeat
unchanged checks. This reuse is not whole-change readiness: current
requirements, affected checks, required CI, and the selected reviewer when
applicable still apply, and a reviewer `PASS` from an older head is not
inherited.

Missing historical evidence is an unknown limitation, not a synthetic result or
a new approval request. It MUST NOT block unrelated authorized preparation. A
current-head reviewer result does not imply that a connector review, semantic
evaluator, repeated broad check, or evidence artifact is needed. Those additions
require an explicit user or repository requirement or a concrete unresolved
risk. Additional testing or review MUST also identify the unmet criterion or
concrete unresolved risk it addresses; a desire for unspecified extra confidence
is not a stopping-condition exception.

The current PR snapshot is authoritative for repository, PR, base, and head
identity. A completion or readiness check MUST consume the current snapshot and
relevant checks; it MUST NOT reconstruct old review history merely because an
older record is absent. The owning child keeps implementation and
review-response ownership, and the parent keeps merge and publication authority.
The selected reviewer MUST NOT be interrupted, replaced, duplicated, or turned
into a second review because a wait is inconvenient.

## Retired review-state artifacts

Review-count, ordered-history, transcript-import, native-recovery, reviewer-
migration, post-cap, final-disposition, and `previous_control_state` inputs are
retired. The runtime rejects them before compact selection, normalization, or
live source reads. Historical records MAY remain immutable for audit or
provenance, but they MUST NOT be executed, rewritten, or used to establish
readiness, completion, merge, or another review. The compact current-head path
above is the only active review-control lifecycle.
