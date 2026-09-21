# Candidate fusion

Use after exploring meaningful alternatives under
[strategy search](strategy-search.md), when combining their strengths could
improve on selecting one candidate. Fusion is optional. MUST preserve the
protected goal, original success criteria, existing authority, and finite
budget. This guide neither redesigns branch management nor applies code or
creates an executor.

## Decide whether to combine

MUST identify the strongest supported elements of the source candidates and
retain each element's source candidate/revision, evidence, assumptions, and
limitations. An expected strength is an estimate until supported by observation;
MUST NOT upgrade it because the source candidate received a favorable score.

Before combining, MUST check:

- **Premises:** can the retained elements' assumptions hold together in this
  task? Mutually exclusive causal explanations are not jointly established.
- **Interfaces:** are inputs, outputs, sequencing, state, and relevant resource
  constraints compatible? Individually sound elements may interact badly.
- **Authority:** does the combination stay within the original scope,
  permissions, ownership, and permitted effects? A component's use elsewhere
  does not transfer authority to this attempt.
- **Expected advantage:** what concrete improvement over selecting the best
  supported single candidate justifies the extra cost and validation?

MUST exclude contradictory or unauthorized elements rather than force them
together to preserve every idea. MUST record retained and excluded elements with
their reasons. If compatibility or expected benefit is unsupported, MUST finish
with a no-fusion decision and the supporting reason. Selecting an existing
candidate or reporting unresolved uncertainty is a valid outcome; no synthetic
fusion candidate is required.

## Treat the combination as a new candidate

When the compatibility and benefit checks support fusion, MUST identify the new
candidate separately from its sources. A compact existing task note can capture
its retained elements, source provenance, excluded elements, interface
decisions, expected advantage, and the new failure modes to test. MUST NOT
require another branch template or a new mandatory artifact.

MUST perform fresh [attempt validation](attempt-validation.md) on the actual
fused candidate, independently assessing observable process and result against
the original criteria. A source candidate's PASS MUST NOT substitute for the
combined candidate's validation. Shared parts may retain correctly scoped
historical evidence, but their interaction and the fused result need their own
relevant checks.

MUST apply [evaluation integrity](evaluation-integrity.md) to affected holdout
checks. A meaningful fused strategy or harness change requires a newly frozen
candidate and uncontaminated evaluation before promotion; MUST NOT inherit a
source's holdout score or expose its private cases to construct the fusion.
Ordinary one-off work retains the existing proportionate verification boundary.

## Keep failures and decisions distinct

If fusion introduces a failure, MUST retain the fused result and original
candidate results separately. MUST identify the evidenced incompatible
interaction or unresolved cause without rewriting the original candidates'
history. A repair or smaller combination is another candidate whose affected
checks must be performed; the budget and authority do not reset.

If relevant validation is unavailable or fails, MUST keep the fused candidate
unverified or failed rather than declaring it better from component scores. A
previously validated source may be selected only when its original evidence is
still applicable to the current task. Otherwise report the remaining gap.

Return either the distinct fused candidate and its validation status, or the
no-fusion decision, with provenance, compatibility/benefit reasons, and limits.
Required changes to the goal or authority go to the orchestrator rather than
being silently adopted. Fusion MUST NOT promote a reusable rule, choose future
learning tasks, or replace review, completion, merge, or publication gates.
