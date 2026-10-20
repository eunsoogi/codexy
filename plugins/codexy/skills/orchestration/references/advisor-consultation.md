# Advisor consultation

## When and where to consult

The Orchestrator MUST consult the Advisor about unresolved cross-Worker design
or ownership conflicts, shared-interface or compatibility choices, contradictory
verification evidence, and unusually difficult decomposition or model decisions
that are not already fixed by a role or host contract. Consultation MAY be
proactive; it MUST NOT depend on a failure count or a native goal-state
threshold. This contract does not define or change those thresholds. Routine
actions with a clear governing rule do not need a second opinion. The caller
MUST NOT ask again only to obtain a preferred answer without new evidence.

A Worker MAY ask for advice about a difficult decision confined to its assigned
issue and owned work. If the decision changes another lane, shared interface, or
ownership, the Worker MUST give the evidence and decision to the Orchestrator;
the Worker MUST NOT make the cross-lane decision or use the Advisor to bypass
that route.

## Ask a bounded question

The caller MUST state the pending decision and constraints, then give the
relevant current code or state, supporting evidence, and any attempted actions
with their observed results, or say when no action has been attempted. It MUST
distinguish observation from inference and identify known unknowns. The question
MUST be bounded to one decision. If a material fact is missing, the caller MUST
ask for that evidence or state the gap rather than presenting an assumption as
fact.

## Evaluate and use the advice

The Advisor MUST give a concrete recommendation supported by decisive evidence,
state material uncertainty, and identify a check or result that could falsify
the recommendation. If the supplied evidence is insufficient, it MUST say what
is missing rather than guess.

Before acting, the caller MUST consider any new evidence that arrived while the
question was outstanding. If that evidence changes the recommendation's basis,
the caller MUST update its decision and may ask one targeted follow-up when the
answer could change the action. Adoption, rejection, or modification MUST follow
the available evidence: adopt advice that fits the requirements and authority,
then run the required verification; reject or modify advice only for a concrete
conflicting requirement, new fact, reproduction, or missing evidence, and name
the supporting evidence for that decision. The caller MUST NOT restart the same
analysis or seek repeated answers only because it retains responsibility for the
decision or wants a different answer without new evidence.

## Preserve authority and review boundaries

Advice MUST NOT expand the assignment, grant permission, transfer ownership,
waive required tests, or change who accepts the work. A Worker remains within
its owned lane; the Orchestrator retains cross-lane decisions and overall
acceptance; any user or parent authority remains in place.

An Advisor consultation MAY help diagnose a reviewer `BLOCK`, but it MUST NOT
replace the selected reviewer, turn `BLOCK` into `PASS`, or clear an unresolved
finding. Keep required verification and review gates open until their existing
contracts are satisfied.

Keep the question, advice, decision and subsequent verification in the existing
task, issue, or review records. MUST NOT create an Advisor receipt schema,
persistent service, or new work-record format.
