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

MUST preserve the plan's concrete nouns and select only one causal assumption
whose failure invalidates acceptance, with the smallest discriminating probe.

For each accepted request, derive the receipt from the supplied plan and
acceptance criteria in this order:

1. Identify the concrete nouns in the input that define the plan's subject and
   success boundary.
2. State one causal assumption that determines whether acceptance succeeds or
   fails.
3. Describe the smallest probe that can distinguish that assumption's passing
   and failing outcomes.
4. Describe the observable passing or failing result and connect it to the
   decision effect without adding an approval or completion judgment.

## Receipt

MUST return exactly these four newline-delimited fields:

```text
invalidating_assumption=<one simple declarative causal claim>
bounded_probe=<one imperative sentence for the smallest discriminating probe>
expected_observable=<one passing observation or one failing observation>
decision_effect=<passing observation keeps the plan, failure stops, narrows, or returns it>
```

MUST NOT add a status, verdict, approval, heading, checklist, or extra field.
MUST NOT fan out work, choose or change an owner, perform the probe, mutate
state, implement a repair, or issue a reviewer or completion judgment.

## Declines

When declining, MUST choose the first matching category in list order and return
exactly its line:

- Routine direct fix:
  `Decline; this is a routine direct fix, not an important plan stress test.`
- Current-diff verdict: `Decline; selected review owns current-diff verdicts.`
- Completion proof: `Decline; proof-driven-completion owns completion evidence.`
- Broad concern list: `Decline; broad concern checklists are out of scope.`
- Multiple plans: `Decline; exactly one plan is required.`
- Owner or worktree choice: `Decline; orchestration owns routing and ownership.`
- Implementation request:
  `Decline; implementation and mutation are out of scope.`
- Approval request:
  `Decline; the skill returns an advisory receipt, not an approval verdict.`
- Missing acceptance criteria:
  `Decline; the required acceptance criteria are missing.`
- No explicit opt-in: `Decline; explicit user opt-in is required.`
