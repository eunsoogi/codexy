---
name: engineering
description: MUST use for diagnosis, specification, domain modeling, test-driven development, refactoring, or quality assurance in one atomic engineering workflow.
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
- [Quality assurance](references/quality-assurance.md) for real-surface proof.

## Shared workflow contract

1. MUST read authorities and diff; MUST keep one outcome and exclusions.
2. MUST record expected/current behavior, riskiest edge, proof, and questions
   before editing.
3. MUST establish faithful pre-change proof. RED/GREEN applies only when
   classified; instruction-only work MUST use readback and MUST NOT manufacture
   RED.
4. MUST make the smallest spec-backed change and preserve public contracts.
5. MUST rerun focused and broader checks and each named authentic surface.
6. MUST clean temporary artifacts and map each changed file to the issue.

## Shared evidence and handoff

Evidence MUST bind input, expected/actual, invocation, cleanup, and state or
head. Narrow checks prove only their boundary; external claims need authentic
proof. MUST NOT hide failures or accept formatting-only LOC reduction. MUST stop
on scope, authority, behavior, or proof conflict.

Handoff MUST name methods, outcome, files, contracts, proof, external results,
cleanup, skips, risks, and next action.
