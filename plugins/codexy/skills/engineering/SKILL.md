---
name: engineering
description: MUST use for diagnosis, specification, domain modeling, test-driven development, refactoring, or quality assurance in one atomic engineering workflow.
---

# Engineering

MUST move one issue-sized outcome from evidence to verified behavior. Select
only applicable methods. Proof-driven completion owns the final audit.

## Method selection

- [Diagnosis](references/diagnosis.md) for wrong or unexplained behavior.
- [Specification](references/specification.md) for unclear outcome or proof.
- [Domain modeling](references/domain-modeling.md) for domain boundaries.
- [Test-driven development](references/test-driven-development.md) only for an
  executable boundary classified `engineering_tdd_required`.
- [Refactoring](references/refactoring.md) for behavior-preserving structure.
- [Quality assurance](references/quality-assurance.md) for real-surface proof.

## Shared workflow contract

1. Read authorities and the diff; keep one outcome and explicit exclusions.
2. Before editing, record expected/current behavior, riskiest edge, proof
   channel, and open questions.
3. Establish faithful pre-change proof. Use RED/GREEN only when classified;
   instruction-only work uses readback and MUST NOT manufacture RED.
4. Make the smallest spec-backed change; preserve public contracts.
5. Re-run focused and broader checks plus every named authentic surface.
6. Clean temporary artifacts; map every changed file to the issue.

## Shared evidence and handoff

Evidence MUST bind input, expected/actual result, invocation, cleanup, and
current state or head. Narrow checks prove only their boundary; external claims
need authentic proof. MUST NOT hide failures or accept formatting-only LOC
reduction. Stop on scope, authority, behavior, or proof conflict.

Handoff MUST name methods, outcome, files, contracts, proof, external
observations, cleanup, skips, risks, and next action.
