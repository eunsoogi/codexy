# Domain modeling

## Method

MUST align names, rules, and ownership before code shape.

1. Build a glossary; identify contexts, owners, states, and adapters.
2. State invariants for transitions, ordering, idempotency, retry, permission,
   ownership, and domain errors.
3. Keep decisions with their owner, translate external shapes at boundaries,
   and prove the rule plus one crossing surface.

## Constraints

- MUST NOT hide rules in helpers, duplicate ownerless invariants, leak payload
  names into core types, or rename concepts from UI copy.
- MUST NOT cross contexts unless scoped; preserve ambiguous terms explicitly.
