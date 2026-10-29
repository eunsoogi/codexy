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

Reviewer-state validity, exact-head binding, and current-head result
interpretation belong to the [review lifecycle](review-lifecycle.md). Overall
evidence validity and outcome judgment belong to
[proof-driven completion](../../proof-driven-completion/SKILL.md); check
selection and verification depth belong to
[quality assurance](../../engineering/references/quality-assurance.md).

One selected reviewer is not an automatic stack. A second reviewer, repeated
broad check, semantic evaluator, connector review, or evidence artifact MUST be
requested only by the user, repository policy, or a concrete unresolved risk.

The executable profile contract remains in the packaged runtime validator.
