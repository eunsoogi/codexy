---
name: plan-stress-test
description: Use when the user explicitly opts in to stress-test one important plan with acceptance criteria before implementation.
---

# Plan Stress Test

Use this skill only after `$orchestration` classifies the request. This is one
read-only advisory challenge, not a routing, review, or completion authority.

## Admission

MUST proceed only when the user explicitly opts in to a stress test and gives
exactly one important plan with acceptance criteria. Otherwise MUST decline
without mutation, probing, or an additional output field.

## Method

MUST identify the one falsifiable assumption whose failure most directly
invalidates the acceptance criteria. MUST choose the smallest time- and
scope-bounded probe that discriminates that assumption before implementation.
MUST state both result states and how each changes the owner's plan decision.

## Receipt

MUST return exactly these four newline-delimited fields with concrete values:

```text
invalidating_assumption=<one falsifiable assumption>
bounded_probe=<one finite discriminating probe>
expected_observable=<the success or failure observations>
decision_effect=<how each observation changes the plan>
```

MUST NOT add a status, verdict, approval, heading, checklist, or extra field.
MUST NOT fan out work, choose or change an owner, perform the probe, mutate
state, implement a repair, or issue a reviewer or completion judgment.

## Declines

When declining, MUST return exactly one matching line:

- Routine direct fix: `Decline; this is a routine direct fix, not an important plan stress test.`
- Missing acceptance criteria: `Decline; the required acceptance criteria are missing.`
- Current-diff verdict: `Decline; selected review owns current-diff verdicts.`
- Completion proof: `Decline; proof-driven-completion owns completion evidence.`
- Broad concern list: `Decline; broad concern checklists are out of scope.`
- Multiple plans: `Decline; exactly one plan is required.`
- Owner or worktree choice: `Decline; orchestration owns routing and ownership.`
- Implementation request: `Decline; implementation and mutation are out of scope.`
- No explicit opt-in: `Decline; explicit user opt-in is required.`
- Approval request: `Decline; the skill returns an advisory receipt, not an approval verdict.`
