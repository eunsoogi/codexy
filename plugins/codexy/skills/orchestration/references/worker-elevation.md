# Worker elevation decision

## Purpose and boundary

This reference lets the Orchestrator assess whether an in-flight default Worker
has discovered enough new hard-work evidence to be considered for elevation.
This procedure MUST remain assessment-only. An elevation decision MUST NOT
create or fork a task, change a model, release issue or worktree ownership, stop
the source Worker, change its goal, expand scope, or approve completion or
review. The complete downstream transition procedures MUST be ready before
actual elevation is executed.

After recording **Elevation may proceed**, the Orchestrator MUST follow the
[parent-loop reconciliation](worker-elevation/reconciliation.md). It MUST
complete and recheck the [source checkpoint](worker-elevation/checkpoint.md)
before preparing a target. The target's read-only setup, identity, authenticated
assignment, parent route, and same-checkout state MUST be confirmed before the
source is retired. A cooperative freeze, eligibility decision, or checkpoint
alone MUST NOT authorize a fork or transfer. The parent MUST separately
authorize the same-issue transition and use the
[transition procedure](worker-elevation/transition.md), including its failure
handling, before changing the existing owner record.

Work identified as hard before assignment MUST continue to use the direct
Terra/Medium route in
[Worker routing](child-routing-policy.md#evidenced-hard-work). This reference
MUST be used only when a general Luna/Max Worker discovers qualifying evidence
after work has begun. Specialists and Workers already assigned to the hard lane
MUST NOT use this path.

## Eligibility conditions

The Orchestrator MUST require all of these conditions before considering
elevation:

- The source MUST be the authenticated current owner of the issue, and its
  assignment record MUST identify it as a `gpt-6-luna` / `max` Worker. The
  Orchestrator MUST keep any host-effective model observation separate; if it is
  not exposed, it remains unknown.
- The source's actual native goal MUST be active and its objective MUST exactly
  match the current `Assignment objective:`. The Orchestrator MUST inspect the
  source goal readback; its own goal or the Worker's status text is not a
  substitute.
- Scope, permissions, inputs, and ownership MUST be resolved, and the existing
  general Worker conditions `fixed_scope`, `low_risk_reversible`, and
  `no_unresolved_decision` MUST remain satisfied.
- Current evidence MUST establish one complete `hard_work` category from the
  existing set:

  | Criterion                  | Required evidence                                                                                                                                              |
  | -------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- |
  | `coupled_contracts`        | At least three distinct executable contracts MUST change together, with an explanation of why changing one independently would violate their shared invariant. |
  | `algorithm_design`         | A new algorithm MUST be derived, with at least two distinct technical constraints and a correctness obligation beyond applying an existing pattern.            |
  | `cross_boundary_diagnosis` | There MUST be at least two affected boundaries, two distinct competing hypotheses, and two concrete observations that discriminate between those hypotheses.   |

The Orchestrator MUST verify these facts against the supplied task evidence. It
MUST NOT treat labels, difficulty keywords, or a structurally complete report as
proof that the facts are true. This policy MUST NOT add a parser for difficulty
wording or create new hard-work criteria. If a request requires automatic
elevation outside these criteria, the Orchestrator MUST report that it needs a
separate maintainer policy decision and MUST NOT apply it through this path.

An actual native blocked goal MUST remain blocked until the explicitly
authorized, source-pair-preserving recovery in
[goal lifecycle](../../goal-lifecycle/SKILL.md) is complete. This assessment
MUST NOT bypass or replace that recovery.

## Evidence and decision

The Worker's existing report to its parent MUST include the original objective
and remaining acceptance criteria, newly discovered hard-work evidence, actual
attempts and their results, the next discriminating check, and the current
branch, worktree, and HEAD. The Worker MUST NOT add a fixed message schema.

The Orchestrator MUST inspect the actual source state and record exactly one
result in the existing task record with its reason and next action:

- **Elevation may proceed** when every eligibility condition is proved. This
  records permission to consider the later transition procedure; it MUST NOT be
  treated as executing that procedure.
- **The existing Worker continues** when the evidence does not justify elevation
  and the Worker can continue safely within its existing scope and permissions.
- **Waiting because conditions are not met** when required state or evidence is
  missing, a prerequisite is unresolved, or the next safe action belongs to the
  Orchestrator. The Orchestrator MUST own that next action and preserve the
  existing owner and conditions.

Transition readiness is not an eligibility condition. The Orchestrator MUST
assess the source state and hard-work evidence above independently of whether a
downstream fork or model-transition procedure is ready. When every eligibility
condition is proved, the result MUST be **Elevation may proceed**, even while
that transition remains unavailable. The Orchestrator MUST record the transition
limitation in the reason and next action, preserve the existing source owner and
conditions, and MUST NOT execute the transition until its separate procedure is
ready. Missing transition support MUST NOT change a proved eligibility result to
**The existing Worker continues** or **Waiting because conditions are not met**.

The Orchestrator MUST reassess if the source state or parent instructions
change. It MUST NOT reuse an earlier decision for changed state or instructions.

## Cases that keep their existing handling

File count, long context, an unfamiliar library, model availability, or failure
count alone MUST NOT justify elevation. Three failures alone MUST NOT trigger an
automatic elevation decision. An intended initial TDD RED MUST NOT count as a
failure event.

Missing inputs or permissions, an unresolved user choice, an owner conflict, a
CI or dependency wait, and a reviewer `BLOCK` MUST retain their existing
handling. They MUST NOT become elevation evidence, and elevation MUST NOT
resolve or override them. When these conditions prevent a safe next action, the
Orchestrator MUST retain the owner and take the next action.

The existing Advisor triggers MUST remain in force: after the third actual
failure in an open repair episode, consult before a fourth speculative repair;
consult immediately for a newly reported blocker; and consult for an unclear
cross-lane decision. The Orchestrator MUST be able to assess sufficient
hard-work evidence without waiting for a failure count, and a clear application
of this policy MUST NOT add an Advisor approval step. If a required consultation
is unavailable, the outcome MUST remain waiting; the Orchestrator MUST preserve
the current owner and conditions and own the next action. Failure episodes,
actionable review findings, and repair budgets MUST continue under their
existing contracts through any later transition; they MUST NOT be reset by a
model change or handoff.

The Orchestrator MUST treat redelivery of the same report as the same event. It
MUST NOT cause a duplicate consultation, failure event, or fork. An elevation
decision MUST NOT imply task completion, review approval, scope expansion,
stopping the source Worker, or a model override. Until the downstream transition
procedures are ready, actual elevation MUST remain unavailable.
