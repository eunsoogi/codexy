# Specification

## Method

MUST turn intent into one observable contract before editing.

1. Extract requirements, exclusions, assumptions, criteria, and open questions.
2. Define one issue-sized outcome and owner; split unrelated work.
3. Define happy-path, riskiest-edge, regression, and applicable external proof.
4. Map every changed file to a requirement and reconcile the final diff and
   current evidence against all criteria.

## Constraints

- MUST NOT edit before the outcome and proof are concrete or widen broad prose
  into adjacent cleanup.
- Evidence proves only its observable and becomes stale when bound state
  changes.
- PR readiness requires current spec proof and clean review feedback.
