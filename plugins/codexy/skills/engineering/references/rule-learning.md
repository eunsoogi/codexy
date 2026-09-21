# Reusable rules and applicability limits

Use this guide to test a reusable rule candidate after an attempt supplies
evidence of a useful process. MUST prefer evidence-supported good processes
under [attempt validation](attempt-validation.md). A lucky outcome from a bad
process, one success, or repeated success on the same example MUST NOT establish
a reusable rule.

MUST preserve the protected goal, original success criteria, authority, and
finite budget. [Strategy search](strategy-search.md) supplies competing
hypotheses; [feedback evidence](feedback-evidence.md) supplies confidence and
provenance. This guide does not choose a future learning curriculum or authorize
background learning, weight training, scheduling, or global memory writes.

## State the rule before testing

MUST identify the proposed rule, application conditions, known exceptions,
supporting process evidence, competing explanation, confidence with reasons, and
conditions that would require narrowing or withdrawal. MUST distinguish observed
support from the proposed generalization. An optional
[rule candidate](../templates/rule-candidate.md) can fit in an existing
authorized note; no permanent storage is required to propose or test a rule.

MUST make the claim narrow enough to test. “Always use this approach” is not
supported merely because it solved the current task. A rule's strength comes
from evidence within its applicability conditions, not its wording or frequency
of repetition.

## Test the capability boundary

MUST choose a small diagnostic task that can distinguish the rule from a
competing hypothesis or reveal a suspected limit. Prefer a task near the
observed capability boundary: neither already trivial nor so difficult that no
useful feedback can be obtained. MUST record the expected behavior before the
test, then the actual behavior, evidence conditions, and generalization result
after it. A diagnostic outside current execution authority remains a proposal.

For scope validation, MUST repeat diagnostics across at least two meaningfully
different situations. Differences must test relevant assumptions, mechanisms, or
boundaries; renamed inputs and duplicate examples do not count as transfer. This
minimum is not proof of universal generalization, statistical superiority, or
reliability beyond the observed sample. MUST keep unsuccessful diagnostics and
uncertainty visible alongside successful ones.

MUST use [evaluation integrity](evaluation-integrity.md) for meaningful strategy
or harness changes, including fresh uncontaminated holdout evidence before
promotion. Author-created diagnostics are development cases, not private
holdouts. MUST NOT inspect private cases to manufacture apparent transfer. Apply
[reward-hacking checks](reward-hacking.md) when evidence supports them. Single
weak feedback, duplicate examples, or development gains with valid holdout
regression MUST NOT justify permanent promotion.

## Track evidence state and disposition

Use these descriptive states; they do not add a runtime schema or automation:

| State           | Meaning and boundary                                                                                                                                              |
| --------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Proposed        | A scoped rule claim exists; supporting hypotheses are not yet validated.                                                                                          |
| Experimental    | Authorized diagnostics are being tried; observations and limits remain explicit.                                                                                  |
| Scope-validated | Repeated, meaningfully different diagnostics support the stated scope and required evaluations do not show unresolved regression. This is not adoption authority. |
| Adopted         | The authorized rule owner accepted the scoped rule in an authorized destination after existing review and proof requirements.                                     |

MUST record the current state and active, narrowed, revised, or withdrawn
disposition with reasons. Confidence may increase with repeated independent
support across different situations, but MUST remain bounded by the tested scope
and evidence quality. State labels alone MUST NOT confer confidence.

On a failed diagnostic, MUST choose a response justified by the evidence: narrow
the application conditions, return to experimental status and retest, revise the
rule as a new candidate, or withdraw it. MUST preserve the original rule,
observations, and failure boundary; changing a rule invalidates affected
validation rather than resetting its history. A failure outside the claimed
scope may refine an exception, but MUST NOT be silently discarded to overstate
generalization.

## Adoption belongs to the destination owner

Before adoption, MUST identify the destination, current write authority, owner,
and existing review and proof requirements. Existing authorization is sufficient
when it covers the write; MUST NOT invent a blanket approval step. Without an
authorized destination or write authority, MUST keep the result a candidate
(experimental or scope-validated as its evidence supports), not an adopted rule.

MUST NOT automatically write global memory, project instructions, user settings,
or model policies. If evidence requires withdrawing an already adopted rule,
MUST record the limit and route the change through that destination's owner and
existing contract. This guide does not itself authorize modifying protected
goals, success criteria, or another owner's stored rules.

Return the rule's tested scope, state, confidence, diagnostic evidence,
generalization limits, disposition, and permitted next action. Selection and
testing can finish without adoption. Missing evidence or permission MUST remain
explicit, not become a claim that the system learned a general capability.
