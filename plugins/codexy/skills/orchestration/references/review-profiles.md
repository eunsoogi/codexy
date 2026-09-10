# Review profiles

The closed profile set selects a proportionate path for the current change:

- `light`: no LLM reviewer; use the relevant direct checks.
- `standard`: use one `codexy-inspector` review when the selected profile or the
  change requires an independent reviewer.
- `strict`: use one `codexy-sentinel` review for a strict or explicitly audited
  lane.

For a child-owned implementation lane, the branch-owning child owns the
profile-selected reviewer when that reviewer is required. The parent consumes
the result and retains merge or publication authority. A separate connector
review is parent-owned only when the user, repository, or a concrete risk
explicitly requires it. These ownership rules MUST NOT create a second default
review or change the selected profile.

The normal reviewer state is current-head evidence. It contains the control
schema, selected profile, policy reviewer when applicable, `reviewed_head`, one
actual `terminal_result` or non-terminal `status`, and `unresolved_findings`. A
reviewer MUST bind the exact current head. `PASS`, `BLOCK`, and `UNOBSERVABLE`
are terminal results; `PENDING` and `RUNNING` are observations of the same
active reviewer and MUST remain pending until its actual result arrives.

Current-head readiness requires the relevant checks for the changed surface, an
actual reviewer `PASS` when a reviewer is selected, and no unresolved actionable
findings. A `BLOCK`, `UNOBSERVABLE`, stale head, failed relevant check, or
actual unresolved finding remains blocking. Fixing a finding may be verified on
the current head without restarting a review count or inventing a new evidence
ledger.

One selected reviewer is not an automatic stack. A second reviewer, repeated
broad check, semantic evaluator, connector review, or evidence artifact MUST be
requested only by the user, repository policy, or a concrete unresolved risk.

Missing historical transcripts, genesis/import records, invocation telemetry,
and quota bookkeeping MUST NOT block the compact current-head path. They remain
unknown evidence and MUST NOT be converted into a synthetic result or a new
approval request.

## Explicit legacy review-state path

Controls that contain `full_review_count`, `delta_review_count`,
`terminal_review_count`, `terminal_review_limit`, `terminal_review_history`,
`pre_pr_import`, `native_history_recovery`, `native_history_provenance`,
`reviewer_migration`, `post_cap_re_review`, or `final_disposition` opt into the
existing legacy transition path. That path is used only when an explicit
requirement or a concrete unresolved risk needs historical reconstruction or a
bounded disposition. Its validators MUST preserve actual reviewer tuples, heads,
findings, source provenance, and authentic GitHub or host evidence; they MUST
reject fabricated history, synthetic verdicts, and caller-supplied authority.

Pre-PR imports and native recovery remain non-admitted until a real current-head
review is available. Post-cap and final-disposition transitions MUST preserve
their actual prior history and continue to enforce code, relevant checks,
ownership, safety, thread, LOC, and merge gates. None of these legacy paths may
waive a real finding or authorize a fourth reviewer.

The executable profile contract remains in the packaged runtime validator.
