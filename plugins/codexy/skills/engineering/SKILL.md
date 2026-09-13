---
name: engineering
description: MUST use for diagnosis, specification, domain modeling, test-driven development, refactoring, performance review, or quality assurance in one atomic engineering workflow.
---

# Engineering

MUST move one issue-sized outcome from evidence to verified behavior. Select
applicable methods only. Proof-driven completion owns final audit.

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

## Change contract and proof choices

Before editing, MUST establish the short
[change contract](references/specification.md#change-contract): requested
behavior, preserved behavior, non-goals, material risks, and evidence needed for
completion. Issue or plan lines are enough for a small change; a separate PRD or
test plan MUST NOT be required by default.

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
5. MUST select the cheapest faithful checks from the changed executable
   boundaries, affected requirements, integration risk, explicit user/repository
   checks, and current evidence. MUST keep required repository CI and run each
   named authentic surface needed for the claimed outcome; broader checks are
   justified when the affected boundary or integration risk reaches them. MUST
   NOT automatically repeat unit, integration, or end-to-end proof when it
   observes the same failure; retain separate levels only for distinct failure
   modes.
6. Before reusing any execution evidence, MUST read back the current diff and
   the relevant implementation, dependency/lock/configuration, fixtures,
   generated inputs, and environment. Each item MUST be classified as unchanged,
   changed, or uncertain for the check being considered.
7. MUST preserve each execution record's original revision, environment, exact
   command, result, and execution state. A later applicability assessment MAY
   explain why that result still covers an unchanged boundary, but MUST NOT
   rewrite its provenance, revision, or claim that the check ran again.
8. A valid passed check MAY be reused for that individual check after the
   applicability assessment when the boundary and environment remain unchanged,
   including an unrelated prose-only or other metadata-only change. A relevant
   implementation, dependency, lock/configuration, shared fixture,
   generated-input, or environment change, a previous failure, an unresolved
   risk, or uncertain impact MUST invalidate the affected evidence and require
   the affected checks again. Integration changes invalidate only affected
   evidence when their impact is known; uncertain integration impact warrants
   broader checks.
9. Within already authorized work, MUST run disposable checks, repair failures
   caused by the change in the assigned scope, and rerun affected checks without
   repeated permission. MUST inspect ambiguous fixture or production effects;
   host denials and authorization for external or destructive operations remain
   authoritative.
10. MUST clean temporary artifacts and map each changed file to the issue.
11. MUST stop when the requirement list, affected checks, and selected review
    when applicable are satisfied with no unresolved in-scope defect. Further
    checks or reviewers require a named unmet criterion or concrete unresolved
    risk. Use the shared
    [execution budget](../orchestration/references/execution-budget.md) and
    [proof-driven completion](../proof-driven-completion/SKILL.md) rules by
    reference; do not add a local quota, evidence ledger, grader, or permission
    gate.

## Shared evidence and handoff

Evidence MUST bind input, expected/actual, original invocation revision,
environment, exact command, result, cleanup, and state or head. When reuse is
proposed, existing handoff fields and prose MUST also explain the current
applicability assessment. Narrow checks prove only their boundary; external
claims need authentic proof. Reusing one check MUST NOT be presented as
whole-change readiness. A child MAY supply valid execution evidence for the
parent to consume after the same applicability assessment; the parent MUST NOT
run a duplicate full suite by default solely to repeat unchanged evidence. The
integrated final state still requires current requirements, affected checks, and
required CI. This policy MUST NOT create an evidence service, hash ledger,
universal receipt schema, new review quota, or extra default reviewer. MUST NOT
hide missing, unobservable, or failed results or accept formatting-only LOC
reduction. MUST stop on scope, authority, behavior, or proof conflict.

Handoff MUST name methods, outcome, files, contracts, proof, external results,
cleanup, skips, risks, and next action.
