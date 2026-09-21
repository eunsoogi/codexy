# Adaptive work

MUST select execution, search, and verification from the task's observed
properties rather than applying one recursive procedure to every request. MUST
preserve the protected goal, original success criteria, native goal contract,
ownership, assigned models, selected review, and actual user and host authority.
These rules guide decisions within those boundaries.

## Task properties

MUST consider all five dimensions before choosing a strategy. A short internal
assessment is sufficient; MUST NOT require a new report, file, runtime field, or
classification schema for every request.

| Dimension     | Values                                  | Effect on the next decision                                                                                                                                                                                           |
| ------------- | --------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Difficulty    | Low / medium / high / unknown           | Low favors one execution followed by evidence-based repair. Medium favors comparison of meaningfully different alternatives. High or unknown favors bounded broad exploration before deep commitment.                 |
| Verifiability | Strong / partial / subjective / unknown | Strong uses direct checks. Partial combines available checks and names gaps. Subjective uses the original criteria and appropriate evidence or human judgment. Unknown first establishes how success can be observed. |
| Duration      | Short / multistep / long                | Short uses a compact execution/check loop. Multistep and long work use bounded stages and proportionate checkpoints under existing handoff and recovery contracts.                                                    |
| Reversibility | Easy / costly / irreversible            | Easy permits authorized reversible experiments. Costly or irreversible actions favor read-only observation, validation, or reversible probes before commitment, retaining existing permission requirements.           |
| Context       | Sufficient / partial / insufficient     | Sufficient supports action. Partial resolves only decision-relevant gaps. Insufficient requires focused observation, questions, or bounded experiments before committing to an unsupported model.                     |

The dimensions MUST be considered together. Low difficulty does not authorize an
irreversible action or excuse missing evidence. Subjective evaluation does not
permit invented certainty. Unknown difficulty is not an unknown runtime workflow
classification and MUST NOT change the closed routing schema.

When premises are uncertain, MUST explore distinguishable problem models or test
the relevant premise before optimizing an execution strategy. MUST NOT ask
unnecessary questions when available evidence already resolves the choice. For
medium difficulty, compare alternatives and validate the selected approach;
combining compatible strengths is optional and requires validation of the new
result. This policy does not implement branch management or candidate fusion.

New evidence may change a property. MUST revise only affected strategy choices
and retain applicable evidence. Reclassification MUST NOT reset budgets,
transfer ownership, create agents or tasks, or relax the goal and success
criteria. Existing explicit-only skill invocation boundaries remain unchanged.

## Contribution check

Before optional additional reasoning or exploration, MUST identify a plausible
contribution to at least one of:

- Meaningfully different solutions or problem hypotheses.
- Reduced uncertainty.
- A test of a decision-relevant premise.
- Better verification.
- Better expected outcome quality against the original success criteria.

A brief rationale is enough; MUST NOT invent precise numerical estimates or a
mandatory record. If none applies, MUST stop that additional reasoning and take
the next supported, authorized action or report the remaining limitation.
Repeating the same analysis because resources remain is not a contribution.

Search, verification, recovery, and replanning MUST stay inside the existing
[execution budget](execution-budget.md). A useful hypothesis does not authorize
unbounded exploration or a new budget. MUST NOT skip mandatory checks for
efficiency, manufacture completion at budget exhaustion, or treat stopping a
search branch as a native goal state transition. When proof remains missing,
MUST retain an incomplete or uncertain outcome under the existing completion and
goal-lifecycle contracts.
