# Self-improvement evaluation

MUST apply the packaged
[evaluation integrity boundary](../../../../plugins/codexy/skills/engineering/references/evaluation-integrity.md)
alongside the existing [skill-evaluation procedure](../SKILL.md). The packaged
boundary owns product evaluation meaning, independence, and promotion; the
parent procedure owns the case matrix, record format, and public storage
boundary. This reference owns only repository execution details for
self-improvement evaluations.

## Before case creation

The independent evaluator MUST freeze the exact candidate revision, environment,
original criteria, and finite budget in its private record before creating
cases. MUST record relevant model, tools, permissions, and baseline conditions.
The author MUST receive only public requirements, rubric, development cases, and
permitted result fields, never private inputs or expected behavior. Case
selection MUST follow the independence criteria in the packaged evaluation
integrity boundary.

The evaluator MUST establish an evaluator-controlled private artifact before
collecting cases. Shared filesystem access alone is not proof of
confidentiality; MUST keep artifacts outside author context and prompts and use
available access controls. If independence or private execution cannot be
established, MUST mark the affected evaluation invalid or needs-review rather
than invent a private run. MUST NOT publish the artifact location.

## Execute and compare

MUST invoke the deployed frozen candidate through the existing procedure and
preserve complete private inputs, expected/prohibited behavior, raw outputs,
exact invocations, measurements, and their sources. MUST distinguish observed
process and outcome validity. Unavailable measurements remain unavailable with
reasons; estimates MUST NOT replace them.

For baseline comparison, MUST follow the packaged evaluation integrity criteria
for same-case, original-criteria, and materially equivalent comparisons. MUST
document condition differences privately and mark incomparable runs invalid or
needs-review. Preserve all failed, interrupted, invalid, and needs-review
records, including their causes.

## Re-evaluation and safe feedback

After a meaningful candidate, strategy, harness, evaluator, or proxy change,
MUST follow the packaged fresh-freeze and contamination rules. A case that
reached the author MUST NOT be reused as a private case. Changed or contaminated
cases and their prior results MUST remain recorded; replacement cases belong to
a new run. Unchanged private cases may support a comparison only under the
packaged boundary's privacy and comparability conditions.

Before releasing feedback, MUST inspect it for leakage. Only case hashes,
status, failed dimensions, and cost summaries may leave evaluator ownership.
MUST NOT include examples, paraphrased answers, diagnostic traces, private
links, exact invocations, or raw measurements that expose cases. If a summary
leaks a case, MUST mark the affected evaluation invalid and arrange a fresh
private run; an edited public summary alone does not restore case secrecy.

For development and holdout divergence, MUST follow the packaged evaluation
integrity promotion criteria and report only the allowed status and failed
dimension. Authors repair from the public rubric and development cases; MUST NOT
lower criteria or remove failed holdout cases. This procedure does not itself
grant merge, publication, or permanent-rule authority.
