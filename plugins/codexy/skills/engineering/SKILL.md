---
name: engineering
description:
  MUST use for diagnosis, specification, domain modeling, test-driven development, refactoring, or
  quality assurance in one atomic engineering workflow.
---

# Engineering

## Purpose

MUST use this skill to move one issue-sized engineering outcome from evidence to verified behavior.
The agent MUST select the applicable sections below; they are one workflow. The agent MUST NOT form
a chain of separately routable skills. Proof-driven completion is the separate final-claim audit.

## Diagnosis

MUST use [Diagnosis](references/diagnosis.md) when behavior is wrong, tests fail, processes hang,
output is unexpected, regressions appear, UI breaks, GitHub automation misbehaves, or a root cause
is unknown.

## Specification

MUST use [Specification](references/specification.md) when a task starts from a PRD, issue,
acceptance criteria, design brief, API contract, user story, or ambiguous feature request that needs
implementation discipline before editing.

## Domain modeling

MUST use [Domain modeling](references/domain-modeling.md) when implementation touches business
concepts, workflows, bounded contexts, domain language, invariants, aggregates, state transitions,
permissions, or cross-module ownership boundaries.

## Test-driven development

MUST use [Test-driven development](references/test-driven-development.md) when the task
classification has set `engineering_tdd_required` for an executable engineering boundary.

## Refactoring

MUST use [Refactoring](references/refactoring.md) when restructuring existing code without changing
behavior, splitting large files or modules, reducing coupling, extracting helpers, simplifying
boundaries, or keeping implementation files at or below the default 250 LOC target.

## Quality assurance

MUST use [Quality assurance](references/quality-assurance.md) when verifying completed work,
designing manual QA, checking real user surfaces, validating release candidates, acceptance
criteria, repository settings, plugin behavior, or PR readiness.

## Selection and boundaries

- MUST start with diagnosis when an observed failure needs reproduction and a root cause.
- MUST start with specification when the requested outcome or proof is not yet concrete.
- MUST apply domain modeling before a change crosses a named ownership or invariant boundary.
- MUST use test-driven development to establish RED, GREEN, and regression proof only for boundaries
  with `engineering_tdd_required`.
- MUST use refactoring only for behavior-preserving structural work.
- MUST use quality assurance to inspect the observable surface after automated proof.
- MUST use only the sections that the atomic outcome requires; they share this one engineering route
  and MUST NOT create a routing chain.
