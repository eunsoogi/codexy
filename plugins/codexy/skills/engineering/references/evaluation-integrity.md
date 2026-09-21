# Evaluation integrity

Use this boundary for a meaningful strategy, harness, evaluator, or proxy change
being evaluated for reuse. Ordinary one-off tasks MUST NOT acquire a mandatory
holdout procedure. MUST preserve the protected goal, original success criteria,
[adaptive work budget](../../orchestration/references/adaptive-work.md), and
[independent process/outcome assessment](attempt-validation.md).

## Separate development and holdout access

Authors may use public requirements, rubrics, and development cases to improve a
candidate. Holdout evaluation belongs to an independent evaluator. The author
MUST NOT create, inspect, or optimize against the private holdout cases,
expected or prohibited behavior, raw results, exact invocations, or
measurements.

The evaluator MUST freeze the candidate revision, environment, criteria, and
finite evaluation budget before creating private cases. It MUST retain those
cases and all complete run records in an evaluator-controlled private bounded
artifact, outside product files and author prompts. The evaluator MUST invoke
the frozen candidate without changing its instructions during the run.

Public feedback MUST contain only case hashes, status, failed dimensions, and
cost summaries. MUST NOT publish private artifact links, inputs, expected or
prohibited outputs, raw responses, exact invocations, measurements, or case
reconstructions to issues, PRs, product guidance, or the author. Public cost
summaries MUST distinguish measured values from unavailable values; MUST NOT
invent missing measurements. Private records retain the exact invocation and
measurement source.

The author may repair a failed dimension using public requirements and fresh
development examples. MUST NOT request a revealing explanation of the private
case or infer and replay a case from a public summary. Evaluators MUST reject
summaries that reveal enough detail to reconstruct a private input or answer.

## Freeze again after meaningful changes

A meaningful strategy, harness, evaluator, or proxy change MUST receive a fresh
candidate freeze and uncontaminated holdout evaluation before promotion. Earlier
results remain attached to their original revision and conditions; MUST NOT
relabel them as a new run. A change during an evaluation invalidates the
affected comparison unless the evaluator can establish which unchanged frozen
artifact was actually invoked.

Leaked or author-exposed cases MUST NOT be reused as private holdout evidence.
The evaluator MUST preserve the original failed or invalid run, mark the
contamination or lost independence, and arrange a fresh evaluation with new
private cases. A changed case is a separate run, not an in-place repair to erase
failure. A case may be reused across an unchanged comparison only while its
privacy, independence, and conditions remain valid.

## Compare fairly

For a comparable baseline, the evaluator MUST record a separate frozen baseline
revision and use the same private cases, success criteria, and materially
equivalent execution conditions for baseline and candidate. MUST record model,
tools, permissions, environment, and budget differences that affect comparison.
Uncontrolled material differences make the comparison invalid or needs-review;
MUST NOT attribute the difference to the candidate alone.

MUST retain failures on both baseline and candidate. Passing relative to a weak
baseline is not proof of the original success criteria, and failing both is not
semantic preservation sufficient for promotion. If development scores improve
while valid holdout performance falls, MUST treat overfitting as a concern and
withhold promotion. MUST NOT lower the rubric, delete difficult cases, or select
only favorable runs to resolve the regression.

## Evaluator and author handoff

The repository's evaluator procedure remains repository-only; a packaged user
does not need that local skill installed. A designated evaluator can implement
this access boundary with its available authorized tools. Role labels alone do
not establish independence, and this document MUST NOT create agents or change
assigned models.

Where the repository skill-evaluation procedure applies, MUST retain all four
private case types with at least two independent cases each: existing-wording
variants, new domains, quotation/negation, and insufficient input. Product
instructions MUST NOT contain their answers. Before author-facing handoff, the
evaluator MUST check the allowed disclosure fields and preserve unsuccessful and
invalid runs privately. If private execution, isolation, or comparison cannot be
established, MUST report the permitted status and failed dimension; MUST NOT
claim successful holdout evaluation.
