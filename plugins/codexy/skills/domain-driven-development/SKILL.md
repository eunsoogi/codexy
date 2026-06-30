---
name: domain-driven-development
description: Use when implementation touches business concepts, workflows, bounded contexts, domain language, invariants, aggregates, state transitions, permissions, or cross-module ownership boundaries.
---

# Domain-Driven Development

## Purpose

Model domain meaning before code shape. MUST keep names, invariants, and ownership
boundaries aligned with the actual workflow so transport, storage, UI, or
framework details MUST NOT leak into core decisions.

## Workflow

1. MUST build a glossary from the request, issue, docs, tests, and existing code.
2. MUST identify bounded contexts:
   - which module owns each term,
   - where external data enters,
   - where domain state changes,
   - which APIs or adapters cross the boundary.
3. MUST capture invariants:
   - required state,
   - forbidden transitions,
   - idempotency and retry behavior,
   - ordering, permission, and ownership rules,
   - explicit domain errors.
4. MUST map code to domain ownership:
   - domain rule in the domain layer,
   - adapter translation at the boundary,
   - UI label or state mapping in presentation,
   - persistence rules in repository or schema code.
5. MUST choose the smallest change that preserves the model.
6. MUST prove behavior at the domain boundary and at one crossing surface when data
   moves through CLI, API, UI, database, queue, filesystem, GitHub, or plugin
   metadata.

## Required Output

```text
Glossary:
Bounded contexts:
Owned invariants:
Boundary adapters:
Domain errors:
Proofs:
Risks:
```

## Gates

- MUST NOT introduce a generic helper if it hides a domain rule.
- MUST NOT validate the same invariant in many places without naming the owner.
- MUST NOT rename domain concepts from UI copy alone.
- MUST NOT refactor across bounded contexts inside a narrow feature branch unless
  the issue explicitly requires it.

## Evidence Rules

- Domain tests MUST name the rule, transition, or invariant being protected.
- Boundary tests MUST prove translation between external shape and domain
  shape.
- MUST run integration or real-surface checks when the risk lives at a boundary
  rather than in pure logic.
- If a domain term is ambiguous, MUST preserve the ambiguity in notes instead of
  silently choosing a meaning.

## Failure Modes

- Leaking API payload names into core domain types.
- Letting database constraints be the only expression of a business invariant.
- Converting a permission or workflow rule into incidental UI state.
- Creating cross-context dependencies that make future atomic work harder.
