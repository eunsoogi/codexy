# Long-work reliability

Use this guide to summarize observed multistep or long-running attempts. It
measures reliability alongside capability without adding global telemetry or new
recovery behavior. MUST consume [attempt validation](attempt-validation.md) and
[feedback evidence](feedback-evidence.md) for verdicts, provenance, and limits.
The existing
[adaptive work budget](../../orchestration/references/adaptive-work.md) remains
authoritative.

## Preserve the observation set

Before aggregating, MUST identify the task scope, success criteria, run unit,
observation window, and relevant execution conditions. MUST retain original
invocations, revisions, environments, sample sizes, interruptions, incomplete or
aborted runs, and unavailable measurements. A later summary MUST NOT rewrite a
run's provenance or claim that a check ran again.

MUST include every observed evaluation run in the declared set. MUST NOT discard
failures, restarts, interrupted runs, or inconvenient revisions to improve a
rate. Separate materially different conditions into labeled cohorts while
keeping the complete set visible. A retry is another attempt, not an overwrite
of the failed original. An unobserved run MUST remain missing evidence, not a
fabricated entry or success.

Use existing authorized task records and the optional
[run summary](../templates/improvement-run.md). MUST NOT require a new log file
for every task or collect hidden reasoning, sensitive raw logs, or global
activity history. Evidence references and minimal observations are sufficient.

## Eight dimensions

MUST report these dimensions with their evidence and unavailable values:

| Dimension             | Observation and denominator                                                                                                                          |
| --------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| Success rate          | Confirmed successful runs / all observed evaluation runs. Keep failed, incomplete, aborted, and unknown outcomes separately visible.                 |
| Failure stage         | Stage where each evidenced failure occurred; unresolved or missing stage stays unknown.                                                              |
| Recovery rate         | Confirmed successful recoveries / attempted recoveries. Distinguish failed, incomplete, and unknown attempts and unattempted or impossible recovery. |
| Repeated-loop rate    | Runs with an observed repeated loop / all observed evaluation runs. Also report event counts when useful and unknown loop observations.              |
| Replan count          | Observed replans per run and total; distinguish no replan from unavailable observation.                                                              |
| Context loss          | Observed lost or unavailable task context, affected stage, and unresolved effect.                                                                    |
| Goal drift            | Evidenced departure from the protected goal or original success criteria, with affected stage and unresolved state.                                  |
| Verifier disagreement | Conflicting verifier judgments and the claim/evidence they disagree about; retain unresolved conflicts.                                              |

MUST define what counts as a repeated loop and successful recovery for the
observed task before counting. A loop means repeated action or state without
relevant progress under that definition, not every authorized retry or planned
iteration. Recovery success requires evidence of the intended recovery outcome;
a recovery attempt starting does not prove success or final task completion.

MUST report numerator and denominator alongside any percentage. If a denominator
is zero, report not applicable or unavailable with the reason, never 100
percent. No attempted recoveries is not perfect recovery. Recovery counts use
attempts, not all failures; the separate unattempted/impossible categories
prevent hiding failures that could not be recovered.

For success and loops, incomplete or unknown runs stay in the total denominator.
MUST label the ratio as confirmed observed events over all runs when
observations are incomplete; it is a lower bound on the event rate, not proof
that unobserved events did not occur. MUST NOT recode unknowns as failures or
zero loops merely to produce a complete distribution. Multiple loops within one
run count once in the run-level numerator; optional event counts remain
separately labeled.

## Interpret without changing state

MUST compare like conditions and state sample limits before claiming reliability
improved. One excellent result, a higher best-case score, or one successful
recovery does not establish a stable success rate. Missing context and verifier
conflict remain evidence limits even if the final artifact looks correct.

When recovery context is needed, consume the current authoritative state read by
[dreaming](../../dreaming/SKILL.md). This summary MUST NOT restore native goals,
transfer ownership, recreate lost state, change blocked policy, or decide
completion. Existing dreaming and orchestration owners choose recovery actions.
MUST NOT infer a current goal or owner from an old run summary.

Return the eight dimensions, denominators, relevant cohorts, unresolved
evidence, and the next permitted observation if one is needed. If no actual
measurement or state is observable, MUST report unknown and its reason. Further
observations stay within existing authority and budget; this guide does not
authorize a background monitor, Watcher changes, or additional recovery
attempts.
