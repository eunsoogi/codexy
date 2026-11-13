# Evidenced hard work

Only a general Worker MAY receive a direct Sol/Medium assignment by supplying
`hard_work` with one closed `criterion` and its complete facts:

| Criterion                  | Required evidence                                                                                                                                                                                                    |
| -------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `coupled_contracts`        | `boundaries`: at least three distinct executable contracts that must change together; `coupling`: why changing one independently would violate the shared invariant.                                                 |
| `algorithm_design`         | `problem`: a new algorithm to derive; `constraints`: at least two distinct technical constraints; `correctness_obligation`: the invariant or proof needed beyond applying an existing pattern.                       |
| `cross_boundary_diagnosis` | `boundaries`: at least two affected technical boundaries; `competing_hypotheses`: at least two distinct plausible causes; `observations`: at least two concrete observations that discriminate between those causes. |

The assigning owner MUST verify these facts against supplied task evidence.
Lists MUST contain distinct nonempty facts; required prose MUST be nonempty. The
parser validates structure, not semantic truth. File count, a general
implementation label, an unfamiliar library, long prose, or model availability
alone MUST NOT count as evidence. Routine edits, bounded summaries,
known-pattern implementations and single-boundary fixes retain Terra/Medium. A
malformed, unknown, or incomplete hard-work object MUST be rejected; missing
hard-work evidence MUST NOT cause escalation. Valid evidence MUST NOT change a
simple, bounded, ambiguous, incomplete, or high-risk request into a general
assignment. Before selecting a hard route, the assigning owner MUST classify any
reported blocker by its cause. Permission limits, missing inputs,
host/tool/environment restrictions, resource contention or capacity, and
unresolved ownership MUST stay with their existing owner or capability path;
they MUST NOT count as hard-work evidence or be bypassed by a different Worker
pair. A separate task-specific implementation fact MAY still satisfy a listed
criterion, but the blocker itself never does. Capability checks MUST apply to
the selected exact pair without substitution.

This is the direct assignment path for work already evidenced as hard. When a
general Terra/Medium Worker discovers qualifying evidence after work begins, the
Orchestrator MUST use the [Worker elevation decision](../worker-elevation.md).
That assessment does not execute a fork or change the Worker's model or
ownership. Before a separately authorized source/target handoff, the parent MUST
follow the
[Worker elevation reconciliation](../worker-elevation/reconciliation.md) and
keep the existing owner until the target's identity, active objective,
authenticated assignment, parent route, same-checkout state, and safe source
retirement are confirmed through supported readbacks. The target remains
read-only until the parent transfers the existing owner record.
