# Evaluator overfitting and reward hacking

Use this check at meaningful optimization checkpoints, after an evaluator or
proxy change, or when evidence suggests that scores and actual goal quality have
diverged. MUST preserve the protected goal and original success criteria. Use
[feedback evidence](feedback-evidence.md) for source/confidence,
[attempt validation](attempt-validation.md) for independent process/outcome
judgments, and [evaluation integrity](evaluation-integrity.md) for private cases
and valid baseline comparisons.

## Look for evidenced signals

MUST distinguish observations from suspicions when checking:

- **Score-only improvement:** development scores rise while evidence of actual
  goal quality stays flat or worsens under comparable conditions.
- **Preference imitation:** candidates increasingly reproduce an evaluator's
  recurring preferences without corresponding support from the original success
  criteria.
- **Rule loopholes:** a candidate satisfies a proxy by bypassing the intended
  task, exploiting a scoring omission, or hiding a failure.
- **Declining discrimination:** the evaluator no longer separates relevant
  strong and weak results as candidates adapt to its preferences.
- **Holdout regression:** development gains accompany degraded valid holdout
  performance on the protected criteria.

A stylistic difference, a single weak criticism, or a noisy score change MUST
NOT be labeled confirmed hacking. MUST record the claim, available evidence,
confidence, and relevant comparison limits. Missing real-quality measurements
remain unknown; they do not prove stagnation. Lower sensitivity in the evaluator
also requires evidence, not merely disagreement with the preferred candidate.

When evidence supports actual quality improvement and criteria remain valid,
MUST NOT suspend useful work merely because the candidate also scores better.
This check detects divergence rather than treating all optimization as suspect.

## Respond to the supported boundary

For supported overfitting or gaming signals, MUST suspend further optimization
of the affected branch against the suspect proxy and withhold rule promotion.
MUST preserve the candidate, observed failures, and reason for suspension.
Suspending a strategy branch MUST NOT pause a native goal, cancel unrelated
authorized work, or change ownership.

MUST select a proportionate stronger evaluation within the existing authority
and finite budget. Depending on the supported signal, this can mean:

- A direct observation of goal quality that tests the apparent proxy gain.
- Independent evidence or an evaluation method that does not share the suspect
  preference or loophole.
- An uncontaminated holdout check on a newly frozen candidate under the existing
  private access boundary.
- An independent evaluator when the risk warrants it and the existing role,
  model, and task authority permits it.
- A review of whether the proxy still measures the original success criteria,
  followed by a corrected method and re-evaluation when authorized.

MUST NOT automatically add a PR reviewer or change assigned models. If an
independent evaluator or required observation is unavailable, MUST report the
limitation and keep the affected judgment provisional; another opinion alone
does not cure correlated evidence.

For weak or ambiguous signals, MUST retain a hypothesis and choose a bounded
discriminating observation under the feedback contract, or report the unresolved
claim when no useful authorized check remains. The orchestrator selects any
further observation outside the current scope; MUST NOT invent proof of gaming
to justify a new process or permission gate.

## Resume only against supported criteria

Before resuming the affected optimization, MUST establish that the corrected or
independent evaluation addresses the observed failure mode and supports the
candidate against the original criteria. MUST retain unresolved limitations. If
the failure mode persists or evidence is unavailable, keep the affected branch
suspended or end it with the recorded reason. Budget exhaustion MUST NOT turn
the candidate into a promoted rule.

A change with improved development scores and degraded valid holdout results
MUST NOT become a promoted rule. A new score after adjusting the evaluator is
not comparable by default; apply the evaluation integrity contract and preserve
the earlier results. MUST NOT remove failed cases or relax criteria to claim a
recovery.

Reviewing a proxy's fidelity is allowed within its existing scope. Changing the
meaning of the protected goal or original success criteria is not: MUST report
any such proposal to the user or goal owner instead of applying it. This guide
MUST NOT create evaluation services, change private-case access, or grant
permanent-rule, mutation, merge, or publication authority.

Return the signal or hypothesis, evidence and limits, affected branch
disposition, selected stronger check, promotion status, and supported next
action. A compact existing task note is enough; no new mandatory artifact is
required.
