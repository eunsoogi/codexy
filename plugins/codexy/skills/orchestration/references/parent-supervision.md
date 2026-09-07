# Parent Supervision

Codex MUST use this reference when an Orchestrator coordinates issue-sized work
through Codex app tasks, a Watcher, Worker callbacks, drift correction, or a
long-lived release goal. It defines evidence and authority boundaries; it does
not create a scheduler, replace goal-lifecycle, or choose a repository's GitHub
policy.

## Canonical role mapping

When this arrangement is authorized, Codex MUST keep these roles distinct:

- The Orchestrator owns judgement, correction, and acceptance. Its `get_goal`
  state MUST remain `null`; it MUST NOT create or recreate a goal.
- The Watcher is an independent same-project app task. It observes and reports
  read-only, and owns the exact long-lived release goal.
- The Worker is a separate app task that owns the implementation branch, files,
  verification, and its finite execution goal.

The Watcher and Worker share model and reasoning effort but are not the same
task or owner. The goal prohibition applies only to the Orchestrator in this
arrangement; the Worker's finite idle-wait and actually observed `blocked`
recovery remain governed by the existing lifecycle.

Configuration metadata, separate from role identity, MUST remain: Orchestrator
and child-to-parent delivery use `gpt-6-astra`/`medium`; Worker, parent-to-child
delivery, and Watcher use `gpt-5.6-luna`/`max`; the configured inspector uses
`gpt-5.6-sol`/`medium`. Every applicable app delivery MUST name its model and
thinking effort.

## Ownership and task surfaces

- Codex MUST keep exactly one implementation owner per issue-sized lane. The
  Orchestrator keeps outcome, correction, and final-acceptance responsibility;
  the Worker owns its branch, files, local verification, and review-response
  fixes.
- Workers and an authorized Watcher MUST be independent Codex app tasks in the
  same saved project. Codex MUST record the project id and actual creating tool.
  A projectless task, a native subagent, or an app API that merely accepts a
  UUID is not a substitute for the required app-task surface.
- The Watcher MUST remain read-only observation. It MAY report a material
  failure, drift, contradiction, scope expansion, missing callback, or
  unavailable channel. It MUST NOT edit worker files, direct or message a Worker
  to change course, supply a repair directive, correct a Worker, decide
  acceptance, verify a correction, replace a Worker, or recruit another Watcher.
- Codex MUST use the canonical role mapping and separate configuration metadata
  above for Orchestrator, Watcher, and Worker routing.
- Codex MUST distinguish app workers and app watchers from native subagents and
  packaged reviewers at the actual creating and readback tools. Codex MUST
  preserve native BLOCK, PASS, UNOBSERVABLE, and closed-handle history and MUST
  NOT fabricate a verdict or reset a review count to fit a newer surface.

## Two observation channels

- Workers MUST send compact gate, fatal-error, and final-result callbacks to the
  Orchestrator when those phases or failures occur. The Watcher observes
  assigned Workers and MUST report only action-required material deltas. A
  callback or Watcher observation alone is a signal, not proof that the work is
  healthy, corrected, or complete.
- Unchanged active-goal reads, routine pre/post/continuation receipts, and
  liveness-only goal-status messages MUST remain internal. The Watcher MUST NOT
  wake the Orchestrator for them; only an actual lifecycle transition, an
  unresolved drift or failure requiring Orchestrator action, missing terminal
  delivery, or a ready external gate may produce a callback or receipt.
- New or changed evidence alone is not notification-eligible. Normal progressing
  work, intermediate successful tests, resolved command mistakes, commits, and
  queued CI MUST remain internal while the Workers are actively progressing. A
  commit or new HEAD alone MUST NOT wake the Orchestrator; it may be retained as
  evidence for a later actionable gate.
- Codex MUST deduplicate one event using a stable event identity before changing
  counters, plan state, or next action. A missed callback, Worker failure,
  Watcher failure, or loss of both channels is an observable limitation; it is
  not permission to invent recovery, an unattended-recovery guarantee, or an
  endless watcher hierarchy.
- Codex MUST use proportionate checkpoints such as plan, first implementation
  slice, and final result only when they help the lane. A concrete risk,
  contradiction, scope expansion, or failure justifies a deeper inspection.
  Codex MUST NOT impose a fixed phase count, universal approval before edits, a
  new mandatory receipt, or exact report wording.
- At a useful checkpoint or after a concrete signal, the Watcher MUST inspect
  the smallest changed artifact, diff, or relevant actual tool call and compare
  it with the currently accepted issue scope, implementation ownership, and
  latest user constraints. It MUST NOT rely only on active/idle state, HEAD, or
  a Worker self-report. The Watcher MUST distinguish ordinary in-scope progress
  from actual drift and report only the material distinction.
- A credible drift report MUST identify the concrete source or call inspected,
  the conflicting current requirement, and the consequence or remaining
  uncertainty in concise natural prose; it MUST NOT require a fixed packet
  schema or contain a repair directive. The Watcher reports only; the
  Orchestrator judges and instructs the Worker, the Worker repairs, and the
  Orchestrator verifies the next relevant source, call, diff, or result. A later
  Watcher observation may report a new mismatch but MUST remain read-only and
  MUST NOT own that correction loop.

## Waiting and direct correction

- Codex MUST prefer cursor-based `wait_threads` with batched targets for
  ordinary app-task waits. Unchanged cursors, bounded timeouts, and legitimate
  long commands are not stalls. Codex MUST NOT emit repeated unchanged status,
  read a full transcript to observe activity, rerun tests only to watch
  progress, or interrupt a live reviewer merely because it is taking time.
- When an actionable signal arrives, the Orchestrator MUST send one grouped,
  actionable correction to the existing worker: observed deviation, smallest
  repair, required evidence, and next permitted step. An acknowledgement is not
  proof. The Orchestrator MUST read back the next relevant actual tool call,
  diff, or result. A cooperative freeze-and-report instruction does not cancel
  an in-flight tool unless the host proves a hard stop.
- Orchestrator fallback inspection MUST require a concrete signal or meaningful
  checkpoint. It MUST NOT become continuous transcript polling. The Orchestrator
  may return control rather than hold a model turn open solely for unchanged
  waiting when a supported Watcher owns that observation.

## Goal ownership and lifecycle

- A user or Orchestrator MAY explicitly authorize a same-project Watcher to
  carry the exact long-lived release goal. The Watcher MUST preserve the exact
  objective text, create it only when its own `get_goal` readback is null or
  complete, and read back `active` before proceeding. If a different unfinished
  goal exists, the Watcher MUST report it and obtain a supported lifecycle
  disposition; it MUST NOT overwrite it or falsely complete it. An observed
  `blocked` state remains governed by the existing `goal-lifecycle` recovery
  authority; this reference MUST NOT replace or restate that recovery sequence.
- In this arrangement, the Watcher is the sole holder of the long-lived release
  goal and the Orchestrator's `get_goal` state MUST remain `null`. The
  Orchestrator MUST NOT call `create_goal` or recreate any goal for setup,
  callbacks, correction, review or merge decisions, or external-event resume;
  after authorized work it MUST return control. This Orchestrator-only exemption
  MUST NOT remove ordinary Worker finite-goal closure or `blocked` recovery.
- A Watcher goal tracks the release outcome; it does not transfer Worker-file
  ownership, Orchestrator correction authority, final judgment, or issue
  completion. Codex MUST record the Orchestrator goal state and Watcher goal
  state separately. A Watcher goal is not evidence that the Orchestrator goal
  was paused, cancelled, transferred, or disabled.
- Outside this arrangement, the existing finite execution lifecycle permits a
  phase to send its idle-wait handoff, complete that finite phase, and leave the
  task idle when only an external event remains; a qualifying wake creates a
  fresh execution goal and current plan before new work. Codex MUST report that
  phase completion separately from any Watcher-held long-lived release goal. It
  MUST NOT use the finite phase transition to claim release, issue, or
  implementation completion. An observed `blocked` record remains governed by
  the existing `goal-lifecycle` recovery authority; this reference MUST NOT use
  ordinary completion language to replace, weaken, or restate that sequence.
  During an unfinished active long-lived goal's external wait or Watcher
  handoff, Codex MUST NOT use administrative completion merely to clear the
  handoff. If the host exposes no cancel, pause, transfer, or objective-update
  operation for that active goal, Codex MUST state the limitation and leave the
  unsupported transition unresolved; it MUST NOT promise that an idle
  Orchestrator will wake later.
- In this arrangement, a qualifying event MUST be handled without creating or
  recreating an Orchestrator goal; the Orchestrator MUST return control after
  the authorized event work. This exception changes only the Orchestrator
  lifecycle and MUST NOT remove ordinary Worker finite-goal closure or `blocked`
  recovery.
- Ordinary app waiting and an explicitly scheduled follow-up are different
  surfaces. Codex MUST NOT create or recreate a heartbeat or automation as a
  workaround for an app Watcher. If no supported Watcher or wake route exists,
  Codex MUST record the exact limitation and bounded fallback and MUST NOT
  invent a monitor identity.

## Recovery, evaluation, and handoff

- Missing output, truncated extraction, or a wrong item-type filter does not
  prove that host evidence is absent. Codex MUST check the actual saved
  artifact, worktree, source record, and creating surface. Codex MUST NOT invent
  a checkpoint path, retry a known-absent path, or restart completed proof after
  compaction without a concrete reason.
- When evaluation of this guidance is explicitly in scope, Codex MUST use the
  existing bounded private skill-evaluation workflow. The evaluator MUST keep
  cases, holdout inputs, expected behavior, exact invocations, raw measurements,
  and complete results in evaluator-controlled private artifacts and MUST
  publish only the allowed hash/status/failure-dimension/cost summary. The
  ordinary supervision loop MUST NOT require a private campaign, answer table,
  fixed case matrix, or unmeasured savings claim.
- When evaluation is in scope, the evaluator MUST use actual Orchestrator,
  Worker, and Watcher calls with the configured model/effort where those roles
  matter and MUST report unavailable telemetry as unmeasured. The evaluator MUST
  choose cases and comparisons proportionate to the changed boundary rather than
  copying a hidden answer. Observed cumulative usage totals MUST NOT be
  presented as priced, cached, or proven savings without the corresponding
  measurement.
- The bounded real-drift/correction evaluation MUST distinguish Watcher-first
  artifact or tool-call detection from a Worker or Orchestrator finding merely
  relayed through the Watcher. Where a real current fault is available, it MUST
  label the actor-separated sequence as Watcher detection/report, Orchestrator
  judgement/instruction, Worker repair, and Orchestrator verification, and
  require the actual report and next relevant readback; it MUST NOT manufacture
  a production fault when none exists. Legitimate in-scope progress and no-drift
  checkpoints MUST control false alarms. The evaluator MUST report detection and
  correction latency, extra reads, and role-separated usage when available;
  missing telemetry remains unmeasured, and no savings or coverage claim may be
  forced.
- Codex MUST finish with current branch, PR state, exact head/base, changed
  files, actual app observations, evaluation limitations, measured versus
  unmeasured usage, retained review history, unresolved findings, and one
  Orchestrator-owned next action. A passing test, callback, Watcher result, open
  PR, or merged change is not a substitute for the other required evidence.
