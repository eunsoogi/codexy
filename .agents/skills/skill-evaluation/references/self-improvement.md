# Self-improvement evaluation

MUST apply the packaged
[evaluation integrity boundary](../../../../plugins/codexy/skills/engineering/references/evaluation-integrity.md)
alongside the existing [skill-evaluation procedure](../SKILL.md). The packaged
boundary defines author/evaluator access; this reference supplies repository
execution details. Neither changes the protected goal or weakens the private
matrix, record fields, or disclosure limits.

## Before case creation

The independent evaluator MUST freeze the exact candidate revision, environment,
original criteria, and finite budget in its private record before creating
cases. MUST record relevant model, tools, permissions, and baseline conditions.
MUST retain the existing four case types and at least two independent cases per
type. The author MUST receive only public requirements, rubric, development
cases, and permitted result fields, never private inputs or expected behavior.

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

For baseline comparison, MUST use the same private cases and original criteria
on a separately frozen baseline under materially equivalent conditions. MUST
document condition differences privately and mark incomparable runs invalid or
needs-review. A failure on both revisions remains a failure against the rubric;
MUST NOT turn it into acceptance through relative scoring. Preserve all failed,
interrupted, invalid, and needs-review records, including their causes.

## Re-evaluation and safe feedback

After a meaningful candidate, strategy, harness, evaluator, or proxy change,
MUST freeze again and use uncontaminated holdout evidence. MUST NOT reuse a case
that reached the author as a private case. Changed or contaminated cases and
their prior results MUST remain recorded; replacement cases belong to a new run.
Unchanged private cases may support a comparison only under the packaged
boundary's privacy and comparability conditions.

Before releasing feedback, MUST inspect it for leakage. Only case hashes,
status, failed dimensions, and cost summaries may leave evaluator ownership.
MUST NOT include examples, paraphrased answers, diagnostic traces, private
links, exact invocations, or raw measurements that expose cases. If a summary
leaks a case, MUST mark the affected evaluation invalid and arrange a fresh
private run; an edited public summary alone does not restore case secrecy.

If development performance rises while valid holdout performance falls, MUST
withhold promotion and report only the allowed status and failed dimension.
Authors repair from the public rubric and development cases; MUST NOT lower
criteria or remove failed holdout cases. This procedure does not itself grant
merge, publication, or permanent-rule authority.
