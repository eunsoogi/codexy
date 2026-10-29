---
name: engineering
description: MUST use for diagnosis, specification, domain modeling, test-driven development, refactoring, performance review, or quality assurance in one atomic engineering workflow.
---

# Engineering

MUST move one issue-sized outcome from evidence to verified behavior. Select
applicable methods only. Proof-driven completion owns final audit.

Engineering owns the technical design, implementation, and verification of one
assigned atomic issue. Orchestration owns classification, ownership, assignment,
worktree, dispatch, execution coordination, and merge; `$planning` owns plan
content, updates, and plan-file rules. Engineering MUST NOT recreate those
authorities.

## Method selection

- [Diagnosis](references/diagnosis.md) for wrong or unexplained behavior.
- [Specification](references/specification.md) for unclear outcomes or proof.
- [Domain modeling](references/domain-modeling.md) for domain boundaries.
- [Test-driven development](references/test-driven-development.md) for an
  engineering boundary whose v2 obligation has `engineering_tests_required`;
  faithful RED/GREEN is required only when that boundary's `tdd_mode` is
  `required`. A v1 `engineering_tdd_required` result is legacy-compatible input
  and MUST NOT be applied as a v2 sequencing mandate.
- [Refactoring](references/refactoring.md) for behavior-preserving structure.
- MUST select [Performance review](references/performance-review.md) only for
  explicit cost or test-efficiency review requests.
- [Quality assurance](references/quality-assurance.md) for real-surface proof.
- [Self-improvement](../self-improvement/SKILL.md) for repeated failure,
  uncertain premises, conflicting verification, or meaningful strategy
  comparison within this outcome. Select only useful contracts; simple questions
  and small well-understood edits MUST NOT acquire a full improvement loop.

## Change contract and proof choices

Before editing, MUST establish the short
[change contract](references/specification.md#change-contract): requested
behavior, preserved behavior, non-goals, material risks, and evidence needed for
completion. Issue or plan lines are enough for a small change; a separate PRD or
test plan MUST NOT be required by default.

An execution-authorized atomic issue MUST proceed through engineering without a
new plan file or an extra planning or review stage; any applicable repository or
profile-selected review remains authoritative. It MUST NOT return to planning
for every technical decision. Engineering MUST propose a change to the parent
plan only when scope, success criteria, or dependencies change; ordinary
implementation choices remain within the assigned issue.

For each changed boundary, MUST decide these independently:

- Behavioral-test need: whether the boundary requires requirement-linked
  behavioral verification.
- Test-first order: whether faithful RED must precede the change.
- Verification depth: the cheapest faithful level and authentic surface needed
  to support the claim.

Test-first order MUST NOT be inferred from the need for behavioral tests, and
verification depth MUST NOT be inferred from either. See
[test-driven development](references/test-driven-development.md) for boundary
obligations and sequencing, and
[quality assurance](references/quality-assurance.md) for depth and surface
selection.

## Applying selected batch results

When
[batch-change-resume.py](scripts/batch_change_resume/batch_change_resume.py)
produces a validated result, use the installed
[batch-change application guide](references/batch-changes.md) and
`batch_change.py` route to apply only an explicit selection of successful items.

- MUST show a readable diff, recheck the original immediately before each
  replacement, replace one independent file at a time, and read back every
  applied result.
- MUST report completed, conflict, and incomplete items separately, preserving
  failed, unselected, and user-changed originals. Repeating an application MUST
  distinguish an already-completed item from a new replacement.
- MUST keep application state in the workspace-local batch state directory, out
  of component inventory and journal state.
- MUST NOT claim multi-file atomicity, protection from arbitrary concurrent
  writers, process resurrection, scheduled wakeups, or automatic commit/push of
  user changes.

## Shared workflow contract

1. MUST read authorities and diff; MUST keep one outcome and exclusions.
2. MUST record expected/current behavior, riskiest edge, proof, and questions
   before editing, using the change contract to anchor any new or replaced test.
3. MUST establish faithful pre-change proof. For v2, every engineering boundary
   with `engineering_tests_required` needs requirement-linked behavioral
   verification; RED/GREEN applies only to that boundary when `tdd_mode` is
   `required`. Optional refactors may use a GREEN or characterization baseline,
   and instruction-only work MUST use proportional readback and MUST NOT
   manufacture RED. Mixed requests follow their boundary obligations.
4. MUST make the smallest spec-backed change and preserve public contracts.
5. MUST use [quality assurance](references/quality-assurance.md) to select
   checks and verification depth, including its test-oracle contract. MUST use
   [proof-driven completion](../proof-driven-completion/SKILL.md) to assess
   execution-evidence validity and reuse and to judge whether requirements are
   proved. MUST use the
   [review lifecycle](../orchestration/references/review-lifecycle.md) for
   reviewer-state rules. Apply each authority per changed boundary.
6. Within already authorized work, MUST run disposable checks, repair failures
   caused by the change in the assigned scope, and rerun affected checks without
   repeated permission. MUST inspect ambiguous fixture or production effects;
   host denials and authorization for external or destructive operations remain
   authoritative.
7. MUST clean temporary artifacts and map each changed file to the issue.
8. MUST stop when the requirement list, affected checks, and selected review
   when applicable are satisfied with no unresolved in-scope defect. Further
   work MUST follow the shared
   [execution budget](../orchestration/references/execution-budget.md). MUST NOT
   add a local quota, evidence ledger, grader, or permission gate.

## Evidence and handoff

Quality assurance owns check selection, verification depth, and test-oracle
preservation. Proof-driven completion owns execution-evidence validity, reuse,
and outcome judgment. Review lifecycle owns reviewer-state rules. These
authorities keep individual check results distinct from whole-change readiness.

Terminal handoff MUST follow
[goal-transition reporting](../orchestration/references/goal-transition-reporting.md);
the completion report MUST follow
[proof-driven completion](../proof-driven-completion/SKILL.md).
