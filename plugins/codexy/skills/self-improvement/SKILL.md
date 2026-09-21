---
name: self-improvement
description: Use when repeated failure, uncertain premises, conflicting verification, or meaningful strategy comparison calls for bounded improvement; not for simple questions or small well-understood edits.
---

# Self-improvement

Improve how the current task reaches and verifies its goal. MUST keep the
protected goal, original success criteria, ownership, assigned models, and
actual authority fixed. This skill changes a supported strategy or evaluation
method; it does not train model weights or grant permission for future work.

## Select only useful steps

Natural-language requests can select this skill without naming it when they
describe repeated failure, uncertain premises, conflicting verification, or a
meaningful comparison of strategies. Explicit `$self-improvement` also selects
it. A simple question or a small, well-understood edit MUST retain the ordinary
execution/check path rather than acquire a full improvement loop. Explicit use
still follows the proportionate path; MUST NOT invent alternatives or a learning
task when they add no useful information.

MUST begin with the existing
[adaptive work policy](../orchestration/references/adaptive-work.md), using the
task's difficulty, verifiability, duration, reversibility, and context to choose
effort. Read only the relevant contracts below; MUST NOT run every step as a
fixed checklist.

| Current need                                                   | Contract and result                                                                                                                                   |
| -------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- |
| Feedback may be weak, repeated, or conflicting                 | [Feedback evidence](../engineering/references/feedback-evidence.md) separates observations, sources, confidence, and limits.                          |
| An attempt or critic's claim needs assessment                  | [Attempt validation](../engineering/references/attempt-validation.md) separates process and outcome and bounds meta-verification.                     |
| The premise or approach needs alternatives                     | [Strategy search](../engineering/references/strategy-search.md) maintains meaningfully different finite branches and pruning reasons.                 |
| Compatible strengths could outperform selection alone          | [Candidate fusion](../engineering/references/candidate-fusion.md) creates and freshly validates a distinct candidate, or declines fusion.             |
| A meaningful strategy/harness change is proposed for reuse     | [Evaluation integrity](../engineering/references/evaluation-integrity.md) separates development from independent private holdout evaluation.          |
| Scores may diverge from real goal quality                      | [Reward-hacking response](../engineering/references/reward-hacking.md) tests the signal and suspends affected optimization when supported.            |
| A reusable rule needs applicability evidence                   | [Rule learning](../engineering/references/rule-learning.md) tests scoped diagnostics and keeps adoption with the authorized destination owner.        |
| Continuing learning is authorized, or a next task is requested | [Learning curriculum](../engineering/references/learning-curriculum.md) chooses an informative diagnostic or returns a proposal.                      |
| Multistep reliability needs assessment                         | [Long-work reliability](../engineering/references/long-work-reliability.md) preserves run denominators, failures, recovery, and unknown observations. |

Within the selected route, identify the candidate or premise, take an authorized
action or observation, assess the evidence, and choose the next supported action
or stopping reason. MUST apply the contribution check before optional additional
reasoning. Repeating the same analysis or having compute left does not justify
another cycle. Missing required proof remains missing when the budget ends.

## Preserve existing owners and boundaries

[`orchestration`](../orchestration/SKILL.md) owns classification, coordination,
ownership, and budget; [`engineering`](../engineering/SKILL.md) owns an atomic
implementation and its verification. They MUST NOT be replaced by this skill.
Plan changes remain with [`planning`](../planning/SKILL.md), current-state
recovery with [`dreaming`](../dreaming/SKILL.md), and final claims with
[`proof-driven-completion`](../proof-driven-completion/SKILL.md).

MUST NOT create goals, app tasks, agents, scheduled jobs, or background learning
merely because this skill was selected. MUST NOT automatically mutate global
memory or stored rules. Existing authorization and destination proof govern any
adoption. New goals and native goal-state transitions retain their own
authority.

[`frame-alternatives`](../frame-alternatives/SKILL.md),
[`plan-stress-test`](../plan-stress-test/SKILL.md), and
[`decision-rationale`](../decision-rationale/SKILL.md) retain their explicit
invocation boundaries. MUST NOT invoke them implicitly as an improvement step.

The core skill requires no optional Devtools, GitHub component, or
repository-only evaluation skill. Use available authorized observations; missing
tools or private evaluation capability MUST remain an explicit limitation. A
tool's absence does not justify inventing evidence or weakening success
criteria.

## Finish with supported progress

Return the selected route, changed strategy or no-change decision, relevant
observed results and limits, and the next permitted action or stopping reason.
Use existing task notes; supporting templates are optional. An improved score,
plausible process, or candidate rule MUST NOT substitute for achievement of the
original goal, observed reliability, or authorized adoption.
