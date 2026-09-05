---
name: frame-alternatives
description: Use when a user explicitly asks to surface credible alternatives for one proposed direction against supplied authoritative constraints.
---

# Frame Alternatives

## Boundary

MUST act only when the user supplies one proposal, its authoritative
constraints, and explicit opt-in to frame-alternatives. Invoking
`$frame-alternatives` explicitly counts as opt-in. Otherwise MUST decline
without producing a receipt.

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

## Noun and constraint tracking

Before drafting a record, MUST build a private ledger rather than outputting
extra fields. Track design nouns (actors, artifacts, operations, and boundaries)
with their roles, then track each constraint's source, modality, scope,
polarity, threshold, and exclusion. Quoted material MUST remain distinct from
the user's assertion, and a negated constraint MUST remain negated.

Equivalent wording MUST normalize to the same ledger meaning only when it keeps
the same actor, relation, scope, modality, polarity, and threshold. A changed
element is a changed constraint, not a synonym. Apply the same normalized ledger
to the current assumption and every alternative; do not add a new YAML key for
the ledger.

## Independence check

Each `credible_alternative` MUST make a genuinely different design choice while
remaining consistent with every normalized constraint. Identify its changed
design axis privately (for example ownership, lifecycle boundary, interaction or
data flow, granularity, timing, or evidence path) and compare that axis with the
current assumption. If the normalized noun roles, relations, and design axis are
unchanged, it is a synonym-only alternative and MUST be replaced or omitted; a
changed axis remains genuinely different even when its wording is removed.

If the proposal, authoritative constraints, or explicit opt-in is missing, or if
no independent alternative can be grounded without inventing a constraint, MUST
decline without producing a receipt. Never fill the maximum with weaker or
repeated alternatives. The ledger and independence check are internal; whenever
a receipt is produced, the response remains the exact four-key YAML receipt
above.
