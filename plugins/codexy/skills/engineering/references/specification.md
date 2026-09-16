# Specification

## Method

MUST turn intent into one observable contract before editing.

1. Extract requirements, exclusions, assumptions, criteria, and open questions.
2. Confirm one orchestration-assigned issue-sized outcome and owner; report
   unrelated outcomes to orchestration rather than splitting or reassigning them
   here.
3. Define happy-path, riskiest-edge, regression, and applicable external proof.
4. Map every changed file to a requirement and reconcile the final diff and
   current evidence against all criteria.

## Change contract

Before editing, MUST record a short contract in the issue or plan:

- Requested behavior: what should be different.
- Preserved behavior: what must remain unchanged.
- Non-goals: adjacent work explicitly left out.
- Material risks: the boundaries or failure modes that could make the change
  unsafe or misleading.
- Evidence needed: the smallest faithful checks and authentic surfaces that can
  prove the requested behavior.

This contract does not by itself request a plan file. When planning is selected,
MUST send plan-content or plan-file changes through `$planning`; engineering
owns the technical design, implementation, and verification within the assigned
issue.

For a small change, these five points may be a few lines. A separate PRD or test
plan MUST NOT be created unless the scope needs one.

## Requirement-led test choices

For every new or replaced test, name the requirement or distinct regression it
protects, the faulty behavior that would fail it, and the source of its expected
result. Expected results MUST come from the agreed contract or an independent
observation, never only from the current implementation output. A test that only
repeats an implementation detail or copies the current output without exposing a
contract violation is not a distinct oracle. Keep this behavioral-test decision
separate from test-first order and verification depth; their boundary-specific
rules live in [test-driven development](test-driven-development.md) and
[quality assurance](quality-assurance.md).

## Constraints

- MUST NOT edit before the outcome and proof are concrete or widen broad prose
  into adjacent cleanup.
- Evidence proves only its observable and becomes stale when bound state
  changes.
- PR readiness requires current spec proof and clean review feedback.
