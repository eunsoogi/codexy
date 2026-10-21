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

## Repeated and chained repair failures

The Orchestrator and Workers MUST use one failure episode for actual failures in
the same repair flow under the task's original completion criteria. A new
affected criterion MUST join the episode when current evidence ties it to that
repair flow. They MUST NOT combine unrelated failures merely because they
occurred in one task. Each Worker MUST keep its own episode for its owned lane;
the Orchestrator MUST handle cross-lane decisions under the existing ownership
route.

The Orchestrator and Workers MUST count the first actual failure or defect
reproduction once. Each distinct new repair or verification attempt that fails
MUST add one event, including an attempt that exposes a different criterion
affected by the same repair flow. A single attempt MUST remain one event even if
several criteria, tool outputs, or messages repeat its result. They MUST NOT
count a reproduction of the same defect without a new repair or verification
attempt, duplicate reports, rereads, unchanged waits, or an intended initial RED
in TDD as new events.

A→A→A and A→B→C each reach the trigger when every letter represents a distinct
failed attempt in the same episode. The third event MUST trigger consultation
before a fourth speculative repair.

The Orchestrator and Workers MUST keep the episode open until all related
original criteria pass. They MUST NOT reset or close it after a model change,
context compaction, handoff, or partial success on one criterion. They MUST
preserve enough existing task or recovery context to recover the observed
failure sequence, affected criteria, partial passes, and last consultation
boundary. They MUST NOT add a runtime counter, permanent database, semantic
prompt parser, receipt schema, or new work-record format.

After the third event, the Orchestrator or owning Worker MUST consult the
Advisor before the next speculative repair, following this reference's question
and advice contract. After each consultation, three further failure events in
the still-open episode MUST trigger another consultation before another
speculative repair; consultation MUST NOT close the episode. The caller MUST
keep consultation and repair work within existing task and execution budgets. If
no budget remains, it MUST stop under the existing budget.

If the Advisor is unavailable when consultation is due, the caller MUST stop
unsupported changes in that episode and report the observed sequence, affected
criteria, and consultation failure to the Orchestrator. A Worker MAY continue
unrelated authorized work; the Orchestrator MUST retain the blocker and report
it to the actual parent or user when appropriate.

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
finding. The caller MUST keep required verification and review gates open until
their existing contracts are satisfied.

The caller MUST keep the question, advice, decision and subsequent verification
in the existing task, issue, or review records. The caller MUST NOT create an
Advisor receipt schema, persistent service, or new work-record format.
