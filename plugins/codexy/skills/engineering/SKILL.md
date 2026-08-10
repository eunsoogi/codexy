---
name: engineering
description: MUST use for diagnosis, specification, domain modeling, test-driven development, refactoring, or quality assurance in one atomic engineering workflow.
---

# Engineering

## Purpose

MUST use this skill to move one issue-sized engineering outcome from evidence to
verified behavior. The agent MUST select the applicable sections below; they are
one workflow. The agent MUST NOT form a chain of separately routable skills.
Proof-driven completion is the separate final-claim audit.

## Diagnosis

MUST use [Diagnosis](references/diagnosis.md) when behavior is wrong, tests
fail, output is unexpected, a process hangs, or the cause is unknown.

## Specification

MUST use [Specification](references/specification.md) when an issue, PRD,
acceptance criteria, or other brief requires an atomic outcome and proof plan.

## Domain modeling

MUST use [Domain modeling](references/domain-modeling.md) when business terms,
workflows, invariants, permissions, state transitions, or ownership boundaries
need explicit modeling.

## Test-driven development

MUST use [Test-driven development](references/test-driven-development.md) before
changing behavior, refactoring, validators, documentation rules, or workflows.

## Refactoring

MUST use [Refactoring](references/refactoring.md) when restructuring behavior
without changing contracts, reducing coupling, or keeping governed files within
the 250-LOC target.

## Quality assurance

MUST use [Quality assurance](references/quality-assurance.md) to prove acceptance
criteria through the real user, maintainer, automation, plugin, or configuration
surface.

## Selection and boundaries

- MUST start with diagnosis when an observed failure needs reproduction and a
  root cause.
- MUST start with specification when the requested outcome or proof is not yet
  concrete.
- MUST apply domain modeling before a change crosses a named ownership or
  invariant boundary.
- MUST use test-driven development to establish RED, GREEN, and regression
  proof for the change.
- MUST use refactoring only for behavior-preserving structural work.
- MUST use quality assurance to inspect the observable surface after automated
  proof.
- MUST use only the sections that the atomic outcome requires; they share this
  one engineering route and MUST NOT create a routing chain.
