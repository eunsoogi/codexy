# Strategy search

Use a finite search when the
[adaptive work policy](../../orchestration/references/adaptive-work.md) calls
for alternative strategies or competing premises. MUST preserve its task
classification, contribution check, authority, and budget. A branch here is a
candidate approach, not a Git branch, app task, or authorization to create one.
This procedure MUST NOT change the explicit invocation boundary of
[`frame-alternatives`](../../frame-alternatives/SKILL.md).

## Create substantive alternatives

For each candidate, MUST capture its core premise, method, expected benefit,
expected failure mode, and substantive difference from the other candidates. Use
an existing authorized note or the optional
[branch shape](../templates/strategy-branch.md); MUST NOT require a database,
new file, or separate agent per branch.

When feasible and useful, compare:

- An improvement of the current strategy.
- A structurally different method, rather than a paraphrase of the same steps.
- A method based on a different causal explanation of the problem.
- An opposite premise when a safe discriminating observation could distinguish
  it from the currently disputed premise.

MUST NOT invent implausible alternatives to fill categories. MUST record why a
material category is unavailable or unhelpful for the current task. When premise
uncertainty dominates, MUST explore competing problem models before optimizing
execution within one model.

MUST compare the mechanism and decisive assumptions, not titles or wording, to
detect duplicate branches. Consolidate semantically equivalent candidates into
one active branch while preserving useful evidence and a stopped-duplicate
reference to the retained candidate. A changed parameter is a distinct branch
only when it tests a materially different prediction or tradeoff.

## Choose what to explore

For each active branch, MUST estimate these dimensions and distinguish estimates
from observations supported by [feedback evidence](feedback-evidence.md):

| Dimension         | Decision question                                                          |
| ----------------- | -------------------------------------------------------------------------- |
| Expected quality  | How well might this satisfy the original success criteria?                 |
| Information value | Which important uncertainty could the next observation resolve?            |
| Novelty           | How different is the mechanism or premise from explored approaches?        |
| Evidence strength | What actually supports it, with what provenance and limitations?           |
| Cost              | What effort or scarce resources would the next step consume?               |
| Risk              | What could go wrong, and is the proposed action reversible and authorized? |

Qualitative estimates are sufficient. MUST NOT fabricate numerical precision or
treat an expected benefit as an observed result. MUST keep risks and authority
visible rather than hiding them inside a combined score.

MUST NOT always expand only the current leader. When a meaningfully different,
promising branch is available within budget, MUST retain at least one alongside
the leader. A less explored branch may deserve the next bounded probe because it
can resolve a key uncertainty, even if its current estimated quality is lower.
Novelty alone does not justify a costly or unsafe probe.

Before expanding, MUST name the next discriminating observation or experiment,
its expected information or quality contribution, and its cost and authority
limits. A retained branch need not run concurrently. MUST NOT create tools,
tasks, permissions, or additional compute merely to keep an alternative alive.

## Stop branches with reasons

MUST stop a branch when its applicable evidence establishes any of these:

- **Refuted premise:** a relevant observation contradicts the core assumption.
- **Repeated failure:** repeated relevant trials fail without a credible new
  repair hypothesis or useful information to gain.
- **Duplicate:** its effective strategy is already represented by another
  branch; identify the retained branch.
- **Exhausted information value:** another step is unlikely to reduce material
  uncertainty, improve validation, or improve the result.

MUST retain the stopped branch's premise, evidence, and disposition reason in
the current authorized task record; pruning MUST NOT erase inconvenient
failures. A refuted premise can justify stopping immediately; MUST NOT repeat
unsafe or pointless trials just to accumulate failures. Weak or conflicting
feedback alone does not establish refutation: apply the feedback contract and
retain uncertainty when evidence is insufficient.

When the finite budget ends, MUST stop expansion and record the budget boundary
separately from evidence that a branch is ineffective. If no viable alternative
remains, MUST state why; MUST NOT fabricate a promising branch or extend the
budget solely to preserve diversity. The orchestrator owns any authorized
substitute observation when the intended experiment is unavailable.

## Handoff

Return the finite active/stopped set, the leader if supported, the materially
different retained alternative if available, observed evidence versus estimates,
and the next permitted action or stopping reason. Candidate fusion is a separate
decision and MUST NOT be implied by retaining multiple branches. A search result
MUST NOT replace final validation, promote a permanent rule, or weaken the
protected goal and success criteria.
