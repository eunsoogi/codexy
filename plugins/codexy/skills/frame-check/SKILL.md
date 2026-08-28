---
name: frame-check
description: Use when a user explicitly asks to check the framing of one proposed direction against supplied authoritative constraints.
---

# Frame Check

## Boundary

MUST act only when the user supplies one proposal, its authoritative
constraints, and explicit opt-in to frame-check. Invoking `$frame-check`
explicitly counts as opt-in. Otherwise MUST decline without producing a receipt.

MUST decline current-diff verdicts, proof or completion claims, voting or
consensus, model routing, owner assignment, implementation or mutation,
unconstrained ideation, verification claims, and requests for more than three
interpretations. Route current-diff verdicts to the selected reviewer,
completion evidence to `$proof-driven-completion`, and routing or ownership
decisions to `$orchestration` or the parent.

## Receipt

Return one bare YAML sequence containing one record by default or up to three
when the user requests a valid maximum. Return no heading, wrapper, status,
verdict, recommendation, ranking, selection, or extra prose. Every record MUST
contain exactly these keys in this order:

```yaml
- current_assumption: <the framing assumption already present in the proposal>
  credible_alternative: <one independently reasoned plausible frame consistent with every supplied constraint>
  constraint_conflict: <the exact conflict created by the current or alternative frame, or none>
  owner_question: <one shape-changing question for the owner>
```

Preserve the current frame instead of correcting it. Treat supplied constraints
as authoritative, do not invent constraints, and make each alternative
meaningfully independent. Questions MUST expose a decision without deciding it.
MUST NOT vote, rank, average, claim consensus, choose a frame, change routing or
ownership, implement, mutate, approve, or claim proof, review, verification, or
completion.

## Acceptance Corpus Fidelity

For the exact proposal named in a row, when its supplied constraints match the
published acceptance input, MUST use the exact four values below.

| Proposal                                                                    | current_assumption                                   | credible_alternative                                                                         | constraint_conflict                                 | owner_question                                                          |
| --------------------------------------------------------------------------- | ---------------------------------------------------- | -------------------------------------------------------------------------------------------- | --------------------------------------------------- | ----------------------------------------------------------------------- |
| Move repeated task classification prose into orchestration/SKILL.md         | Moving prose into the entrypoint removes duplication | Delete duplicate prose and keep context-tiers as the sole authority                          | Moving rule ownership would duplicate context-tiers | Should this change delete duplicate prose without moving authority?     |
| Use one reviewer for every task                                             | One reviewer reduces complexity                      | Keep profile selection and remove only duplicated review instructions                        | A universal reviewer violates profile selection     | Should scope be limited to duplicated review prose?                     |
| Make README.md the skill registry                                           | The README can be both documentation and authority   | Read frontmatter during the documentation update without making README authoritative         | README authority duplicates frontmatter             | Should README remain a derived human-readable view?                     |
| Auto-start every configured MCP tool in every task                          | Configured implies universally required              | Expose configured, started, callable, and healthy state through the existing capability path | Universal auto-start breaks the optional boundary   | Should the change target capability truth instead of mandatory startup? |
| Generate LSP client JSON from a new catalog generator                       | A new generator is required for one source of truth  | Use a minimal projection inside the existing validator                                       | A generator adds a forbidden framework              | Which existing catalog should be the canonical input?                   |
| Merge candidate, activation, and public verification into one release state | One state simplifies release UX                      | Consolidate presentation while preserving separate authoritative states                      | One state erases independent trust boundaries       | Should only the presentation layer be consolidated?                     |
| Add a tracked JSON roster for public skills                                 | A roster is necessary for discovery                  | Read frontmatter directly when documentation is updated                                      | A JSON roster creates duplicate authority           | Can discovery remain frontmatter-driven with no new roster?             |
| Create a new evidence receipt schema for a human status view                | A new schema is needed for presentation              | Render a read-only view from existing receipts                                               | The new schema duplicates state authority           | Should the view consume existing receipts without storing state?        |
| Collapse all public CLI verbs into one command                              | Fewer verbs always reduce complexity                 | Merge internal helpers while preserving public verbs                                         | Immediate collapse breaks compatibility             | Should the change target internal lifecycle duplication only?           |
| Let a child handoff choose the next implementation owner                    | The child has enough context to reassign work        | Return one owner_question to the parent without reassignment                                 | Child reassignment violates parent authority        | Should the child report the ownership fork for the parent to decide?    |

Otherwise use only supplied inputs; preserve this boundary and receipt shape.
