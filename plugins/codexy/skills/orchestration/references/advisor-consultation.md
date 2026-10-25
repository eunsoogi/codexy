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

## A new child blocked report

When a child reports a new blocker, the Orchestrator MUST inspect the actual
obstacle and authoritative current state, then request one bounded Advisor
consultation before deciding its disposition. This trigger MUST NOT wait for
three failures. The Orchestrator MUST deduplicate the event under
[parent supervision](parent-supervision.md); repeated wording or an unchanged
wait MUST NOT trigger another consultation, instruction, or failure event.

The Orchestrator MUST distinguish the following from the child's report and
available native goal readback, selected review result, and dependency evidence.
It MUST NOT infer native goal state from prose or substitute its own goal
readback for the child's. Missing child-state evidence MUST remain unknown and
limit the proposed action until the owner supplies the authoritative readback.

- Blocked wording with an active goal MAY describe an actionable technical
  obstacle; it MUST NOT cause a goal transition by itself.
- Native `status=blocked` MUST stop child implementation until the explicitly
  authorized recovery in [goal-lifecycle](../../goal-lifecycle/SKILL.md) is
  satisfied. Advice MUST NOT clear it with `complete`, treat it as active, or
  invent fork/archive/resume authority. That recovery MUST preserve the original
  objective, branch/worktree, source settings, and exactly one active owner.
- Reviewer `BLOCK` MUST retain the selected review and unresolved findings;
  consultation MAY propose a repair but MUST NOT change the verdict.
- A dependency wait MUST retain its real producer and wake condition. If the
  parent can advance it within existing authority, it MUST identify that parent
  action and the next child action; an unresolved external dependency MUST NOT
  become a fabricated terminal goal transition.
- An unanswered material user decision, missing user information, or missing
  permission MUST retain the exact unresolved boundary. The Orchestrator MUST
  report the needed decision or authority and execution condition to the user
  or responsible owner, and continue only independent authorized work.

The consultation MUST carry the original objective and success criteria; the
child's identity, owned scope, branch/worktree, and goal-state evidence; actual
attempts, failures, partial passes, and current evidence; user and host
constraints; known unknowns; and possible next steps. The question MUST ask for
the likely cause, the smallest progress path within existing authority, missing
evidence, and a verification result that could disprove that path. The Advisor
MUST remain read-only and MUST NOT take ownership or make the final decision.

After advice arrives, the Orchestrator MUST compare it with current evidence,
including any new user direction, and the actual authority. It MUST record the
chosen action and reason under the advice-evaluation rules below. When progress
is authorized and possible, it MUST send the existing child one grouped
instruction naming the specific smallest repair or diagnostic action, required
verification, next allowed phase, and stop condition. A generic "continue"
message MUST NOT substitute for that instruction. Native blocked-goal recovery
MUST use its lifecycle route before implementation; other cases MUST retain the
same child owner. The Orchestrator MUST reconcile capacity before any resume
under [supervised app-child capacity](parent-supervision.md#supervised-app-child-capacity).

The Orchestrator MUST confirm the child's actual next relevant call, diff, or
verification result against that instruction before claiming resumption or
resolution. Advice, message acceptance, and child acknowledgement alone MUST
NOT prove recovery. Handoffs and external waits MUST continue to follow
[goal-transition reporting](goal-transition-reporting.md); this procedure MUST
NOT add a parent-reply requirement or complete an unfinished phase.

For the same incident, the Orchestrator MUST reuse an accessible Advisor under
the reuse rules below. A follow-up MUST have new evidence, a distinct actual
failure, or changed constraints; unchanged reports MUST NOT restart the loop.
If the Advisor or a required messaging/recovery tool is unavailable, it MUST
report the exact limitation once, retain ownership and the unmet condition, and
stop unsupported changes. It MUST NOT implement an Advisor substitute, create
an arbitrary heartbeat, or repeat unchanged continuation turns. Issue
registration alone MUST NOT authorize contacting unrelated live tasks or
starting new children. Recovery MUST NOT shrink the objective, weaken checks,
change models, patch a child's files from the parent, or create duplicate owners.

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

## Reuse an Advisor for a related follow-up

When a follow-up concerns the same pending decision or still-open failure
episode, the caller MUST reuse the same accessible Advisor identity if its
context and authority remain applicable. An idle or completed response does not
by itself make that identity unavailable or justify starting a duplicate
Advisor. A later consultation trigger in the same open episode follows this
reuse rule too.

The caller MUST send a bounded update containing new evidence, actions and
results since the last consultation, whether and how its basis changed, and the
next question. It MUST retain applicable constraints and context without
replaying the full transcript or unchanged evidence. The caller MUST identify
new observations separately from prior advice and its own inferences.

The caller MUST start a fresh consultation when the question has materially
different scope, the existing context is stale or conflicts with current
evidence, independent judgment is required, or the existing identity cannot be
reached. When only the handle is unavailable and the Advisor role remains
available, the caller MUST carry relevant current evidence and constraints into
the fresh consultation and identify any prior recommendation as historical. If
the Advisor itself cannot be called, the caller MUST follow the existing
unavailable-Advisor fallback above and stop unsupported changes.

A historical recommendation remains consultation history, not current proof.
Terminal delivery alone MUST NOT trigger a duplicate consultation; the caller
MUST use the existing identity for a related follow-up when it remains
accessible. Reuse MUST preserve the open failure episode, caller ownership, and
selected review. It MUST NOT relax the Advisor's read-only and no-implementation
boundary: the Advisor MUST NOT create, edit, delete, rename, or stage files, run
commands that mutate repository state, take implementation ownership, or
redelegate work. An Advisor consultation remains separate from the selected
reviewer and MUST NOT waive verification or review findings.

This reuse rule describes bounded context sharing by the caller; it does not
prove provider-side caching or reduced prefill, token use, or cost. Such claims
MUST be based on actual provider measurements.

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
