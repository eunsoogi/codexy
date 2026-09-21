# Choosing the next learning task

Use this guide when bounded continuing improvement is within the current task's
scope, or when asked to propose the next diagnostic task. MUST preserve the
protected goal, original success criteria, execution authority, and
[adaptive work budget](../../orchestration/references/adaptive-work.md).
Selection can finish with a proposal; it does not authorize execution.

Consume current [strategy evidence](strategy-search.md) and
[rule applicability evidence](rule-learning.md). MUST NOT infer capability from
an unsupported self-rating or treat an adopted rule as universally valid. The
observed successes, failures, uncertainty, and available feedback define the
current learning boundary.

## Choose for information value

MUST identify the uncertainty or capability boundary the next task would test.
Prefer tasks that can:

- Expose an uncertain premise relevant to the original goal.
- Distinguish competing strategies through different predicted outcomes.
- Test a recently proposed or learned rule under a relevant new condition.
- Probe an evidenced failure boundary without merely repeating the failure.
- Reach slightly beyond demonstrated capability while still yielding useful,
  observable feedback.

MUST name the expected observation and how its possible results would change the
current decision, hypothesis, or rule scope. A novel topic alone is not useful
information. Estimated learning value MUST remain an estimate until the task
produces evidence.

Compare a small finite set using expected information, relevance, feedback
availability, cost, risk, and authority. MUST NOT manufacture numerical scores
or expand the search solely because compute remains. When a small test can
distinguish the hypotheses, prefer it over an expensive task with the same
information contribution.

## Keep difficulty near observed capability

MUST avoid spending most of the learning effort on already stable successes or
tasks so difficult that the system cannot obtain useful feedback. A simple
control case or a deliberately difficult boundary probe may still be useful when
it resolves a specific uncertainty; difficulty alone does not decide value.

When evidence of capability improves, move the diagnostic boundary outward in
small relevant steps. When failures or missing feedback show the task was too
hard, reduce complexity or isolate one uncertain premise so the next result can
be interpreted. MUST retain the prior failed attempts and conditions; choosing
an easier diagnostic MUST NOT lower the original success criteria or erase the
unsolved goal.

If capability evidence is uncertain, choose a small discriminating diagnostic
instead of assuming either mastery or inability. MUST distinguish lack of
capability from missing context, unavailable tools, or absent observation before
raising or lowering difficulty on that basis.

## Return one bounded next step

Return the selected task, relevant hypothesis or rule, observed capability
basis, expected behavior and discriminating feedback, expected information
value, cost/risk, authority, and stop condition. A short existing task note is
enough; no new curriculum database or mandatory template is required.

When execution is authorized, the next task still follows the existing
engineering, verification, and rule-learning contracts. When it is outside scope
or requires unavailable authority, MUST return a proposal and the relevant limit
without executing it. The orchestrator or goal owner decides any scope
expansion. Selection MUST NOT create goals, app tasks, scheduled jobs, or
background learning; it MUST NOT adopt or revise stored rules itself.

If no candidate offers useful information within the current boundary, MUST end
selection and record why. Remaining compute is not a reason to generate more
tasks. A useful but unaffordable or unauthorized task can remain a proposal, not
an active learning commitment. Replanning or selecting a new difficulty MUST NOT
reset the finite budget.
