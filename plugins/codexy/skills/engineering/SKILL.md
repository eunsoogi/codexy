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
- [Test-driven development](references/test-driven-development.md) only for an
  executable boundary classified `engineering_tdd_required`.
- [Refactoring](references/refactoring.md) for behavior-preserving structure.
- MUST select [Performance review](references/performance-review.md) only for
  explicit cost or test-efficiency review requests.
- [Quality assurance](references/quality-assurance.md) for real-surface proof.

## Shared workflow contract

1. MUST read authorities and diff; MUST keep one outcome and exclusions.
2. MUST record expected/current behavior, riskiest edge, proof, and questions
   before editing.
3. MUST establish faithful pre-change proof. RED/GREEN applies only when
   classified; instruction-only work MUST use readback and MUST NOT manufacture
   RED.
4. MUST make the smallest spec-backed change and preserve public contracts.
5. MUST select checks from the changed executable boundaries, affected
   requirements, integration risk, explicit user/repository checks, and current
   evidence. MUST keep required repository CI and run each named authentic
   surface needed for the claimed outcome; broader checks are justified when the
   affected boundary or integration risk reaches them.
6. MUST reuse an adequate existing result only while the relevant boundary,
   environment, and evidence remain valid. MUST rerun it when a relevant change,
   failure, changed environment, or unresolved concern invalidates it; MUST NOT
   treat older evidence as current only because it passed.
7. Within already authorized work, MUST run disposable checks, repair failures
   caused by the change in the assigned scope, and rerun affected checks without
   repeated permission. MUST inspect ambiguous fixture or production effects;
   host denials and authorization for external or destructive operations remain
   authoritative.
8. MUST clean temporary artifacts and map each changed file to the issue.

## Shared evidence and handoff

Evidence MUST bind input, expected/actual, invocation, cleanup, and state or
head. Narrow checks prove only their boundary; external claims need authentic
proof. MUST NOT hide failures or accept formatting-only LOC reduction. MUST stop
on scope, authority, behavior, or proof conflict.

Handoff MUST name methods, outcome, files, contracts, proof, external results,
cleanup, skips, risks, and next action.
