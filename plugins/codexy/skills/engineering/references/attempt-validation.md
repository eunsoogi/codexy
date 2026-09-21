# Attempt validation

Use this contract to assess one attempt against the protected goal and original
success criteria. MUST consume [feedback evidence](feedback-evidence.md) for
provenance, confidence, conflicts, and safe observations; MUST NOT redefine its
source or confidence rules here. The existing
[adaptive work policy](../../orchestration/references/adaptive-work.md) owns
effort and stopping boundaries.

## Two independent axes

MUST assess each axis from its own relevant evidence:

- **Process validity:** were observable decisions, inputs, tool use, changes,
  and supporting evidence valid under the applicable constraints? Check whether
  the stated justification follows from evidence and whether claimed causal
  links have support. MUST NOT request, collect, or reconstruct hidden
  chain-of-thought.
- **Outcome validity:** does the observed result satisfy the original goal and
  success criteria on the required surface? A plausible approach, favorable
  critic, or intermediate score MUST NOT substitute for that result.

For each axis, MUST record good, bad, or unknown, the supporting evidence, and
any provisional qualification. Good requires relevant affirmative support; bad
requires an evidenced violation or failure. Missing or inconclusive evidence
MUST remain unknown rather than becoming either good or bad. If a judgment
depends on unverified evidence, MUST mark it provisional and name the unresolved
claim. A provisional or unknown axis MUST NOT be reported as verified success.

| Process | Outcome | Interpretation                                                                                        |
| ------- | ------- | ----------------------------------------------------------------------------------------------------- |
| Good    | Good    | A supported successful attempt within its tested scope.                                               |
| Good    | Bad     | The method may be sound, but the goal was not achieved; investigate the failed outcome.               |
| Bad     | Good    | The result may be accidental; preserve the process defect and do not learn it as a successful method. |
| Bad     | Bad     | Both the process defect and failed result require attention.                                          |

MUST NOT infer one axis from the other. Where either is unknown or provisional,
MUST report that state alongside any established axis instead of forcing the
attempt into a verified quadrant. Reusable rule candidates should come from
evidence-supported good processes; a good/good verdict alone MUST NOT promote a
permanent rule or prove generalization.

## Separate the validation functions

These are functions, not a requirement to create four agents or change assigned
models. MUST preserve the existing reviewer selection and ownership contract.

1. **Generator:** produces a candidate and identifies the goal and criteria it
   is intended to satisfy.
2. **Critic:** states strengths and weaknesses as claims with relevant evidence
   references. Unsupported criticism MUST remain a hypothesis, not a fact or an
   automatic repair instruction.
3. **Verifier:** checks whether each material critical claim is supported by
   actual evidence under the right scope and conditions. MUST distinguish
   supported, contradicted, and unresolved claims and preserve their source
   provenance. An unchecked verifier assertion MUST remain provisional; the
   title of the role does not validate it.
4. **Meta-verifier, when justified:** checks the verifier's evidence selection,
   criteria, and premises for a named material uncertainty. It MUST NOT replace
   original success criteria with evaluator preferences.

When feasible within current authority and budget, use different evidence
sources, tools, prompts, or evaluation methods across these functions to reduce
shared errors. Different models may be used only when already permitted by the
assigned model policy. MUST NOT treat different role names or repeated readings
of one source as independent corroboration. A single agent may perform the
functions with clearly separated evidence checks when extra agents are neither
required nor authorized.

## Bound meta-verification

Conflicting evidence, high-impact decisions, changed evaluators, or suspected
gaming can justify checking a verifier's premises. MUST first name the concrete
uncertainty, the smallest discriminating check, and the existing budget and
stopping condition. A verifier saying “verified” is not sufficient evidence, but
that alone MUST NOT trigger an endless verifier chain.

Normally use at most one additional meta-verification level. MUST NOT
recursively add levels by default. Any exceptional further check needs a
concrete unresolved material risk and must fit the existing authority and finite
budget; resource availability alone is not a reason. Prefer a direct
authoritative observation over another opinion when it can settle the claim.

If the meta-check finds a false premise or unsuitable criterion, MUST reopen the
affected judgment, preserve the earlier finding and its limitation, and reassess
from corrected evidence. If uncertainty remains at the stopping condition, MUST
return a provisional or unknown verdict and the unresolved claim to the
orchestrator. MUST NOT silently pass, weaken the goal, or manufacture confidence
because further checking is unavailable.

## Compact verdict

Use the optional [attempt verdict](../templates/attempt-verdict.md) within an
existing authorized note when helpful. MUST bind the verdict to the actual
attempt/revision and evidence conditions. MUST NOT require a new file or hidden
reasoning transcript per attempt. Completion, review, merge, and publication
retain their existing gates; this verdict supplies evidence rather than
replacing those decisions.
