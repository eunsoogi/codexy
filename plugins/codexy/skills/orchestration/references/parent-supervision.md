# Parent Supervision

Codex MUST use this reference when a parent coordinates issue-sized work
through Codex app tasks, an observation watcher, worker callbacks, drift
correction, or a long-lived release goal. It defines evidence and authority
boundaries; it does not create a scheduler, replace goal-lifecycle, or choose a
repository's GitHub policy.

## Ownership and task surfaces

- Codex MUST keep exactly one implementation owner per issue-sized lane. The parent keeps
  outcome, correction, and final-acceptance responsibility; the child owns its
  branch, files, local verification, and review-response fixes.
- Implementation workers and an authorized watcher MUST be independent Codex
  app tasks in the same saved project. Codex MUST record the project id and actual
  creating tool. A projectless task, a native subagent, or an app API that merely
  accepts a UUID is not a substitute for the required app-task surface.
- The watcher MUST remain read-only observation. It MAY report a material failure,
  drift, contradiction, scope expansion, missing callback, or unavailable
  channel. It MUST NOT edit worker files, correct a worker, decide acceptance,
  replace a worker, or recruit another watcher.
- Codex MUST preserve the fixed role contracts: parent and child-to-parent delivery use
  Astra/medium; ordinary workers, parent-to-child delivery, and the watcher use
  Luna/max; the configured inspector remains Sol/medium. Every applicable app
  delivery names its explicit model and thinking effort.
- Codex MUST distinguish app workers and app watchers from native subagents and packaged
  reviewers at the actual creating and readback tools. Codex MUST preserve native
  BLOCK, PASS, UNOBSERVABLE, and closed-handle history and MUST NOT fabricate a
  verdict or reset a review count to fit a newer surface.

## Two observation channels

- Workers MUST send compact gate, fatal-error, and final-result callbacks to the
  parent when those phases or failures occur. The watcher observes assigned
  workers and MUST report only material deltas. A callback or watcher observation
  alone is a signal, not proof that the work is healthy, corrected, or complete.
- Codex MUST deduplicate one event using a stable event identity before changing counters,
  plan state, or next action. A missed callback, worker failure, watcher failure,
  or loss of both channels is an observable limitation; it is not permission to
  invent recovery, an unattended-recovery guarantee, or an endless watcher
  hierarchy.
- Codex MUST use proportionate checkpoints such as plan, first implementation slice, and
  final result only when they help the lane. A concrete risk, contradiction,
  scope expansion, or failure justifies a deeper inspection. Codex MUST NOT
  impose a fixed phase count, universal approval before edits, a new mandatory
  receipt, or exact report wording.

## Waiting and direct correction

- Codex MUST prefer cursor-based `wait_threads` with batched targets for ordinary app-task
  waits. Unchanged cursors, bounded timeouts, and legitimate long commands are
  not stalls. Codex MUST NOT emit repeated unchanged status, read a full
  transcript to observe activity, rerun tests only to watch progress, or
  interrupt a live reviewer merely because it is taking time.
- When a material signal arrives, the parent MUST send one grouped, actionable
  correction to the existing worker: observed deviation, smallest repair,
  required evidence, and next permitted step. An acknowledgement is not proof.
  The parent MUST read back the next relevant actual tool call, diff, or result. A cooperative
  freeze-and-report instruction does not cancel an in-flight tool unless the
  host proves a hard stop.
- Parent fallback inspection MUST require a concrete signal or meaningful
  checkpoint. It MUST NOT become continuous transcript polling. The parent may
  return control rather than hold a model turn open solely for unchanged waiting
  when a supported watcher owns that observation.

## Goal ownership and lifecycle

- A user or parent MAY explicitly authorize a same-project watcher to carry the
  exact long-lived release goal. The watcher MUST preserve the exact objective
  text, create it only when its own `get_goal` readback is null or complete, and
  read back `active` before proceeding. If a different unfinished goal exists,
  the watcher MUST report it and obtain a supported lifecycle disposition; it
  MUST NOT overwrite it or falsely complete it. An observed `blocked` state
  remains governed by the existing `goal-lifecycle` recovery authority; this
  reference MUST NOT replace or restate that recovery sequence.
- In watcher-supervised Astra-parent mode, the watcher is the sole goal holder
  for the release and the Astra/medium parent's `get_goal` state MUST remain
  `null`. The parent MUST NOT call `create_goal` or recreate any goal for setup,
  callbacks, correction, review or merge decisions, or external-event resume;
  after authorized work it MUST return control. This parent-only exemption MUST
  NOT remove ordinary worker finite-goal closure or `blocked` recovery.
- A watcher goal tracks the release outcome; it does not transfer worker-file
  ownership, parent correction authority, final judgment, or issue completion.
  Codex MUST record the parent goal state and watcher goal state separately. The
  watcher goal is not evidence that the parent goal was paused, cancelled,
  transferred, or disabled.
- Outside watcher-supervised Astra-parent mode, the existing finite execution
  lifecycle permits a phase to send its idle-wait handoff, complete that finite
  phase, and leave the task idle when only an external event remains; a
  qualifying wake creates a fresh execution goal and current plan before new
  work. Codex MUST report that phase completion separately from any
  watcher-held long-lived release goal. It MUST NOT use the finite phase
  transition to claim release, issue, or implementation completion. An observed
  `blocked` record remains governed by the existing `goal-lifecycle` recovery
  authority; this reference MUST NOT use ordinary completion language to
  replace, weaken, or restate that sequence. During an unfinished active
  long-lived goal's external wait or watcher handoff, Codex MUST NOT use
  administrative completion merely to clear the handoff. If the host exposes no
  cancel, pause, transfer, or objective-update operation for that active goal,
  Codex MUST state the limitation and leave the unsupported transition
  unresolved; it MUST NOT promise that an idle parent will wake later.
- In watcher-supervised Astra-parent mode, a qualifying event MUST be handled
  without creating or recreating a parent goal; the parent MUST return control
  after the authorized event work. This exception changes only the parent
  lifecycle and MUST NOT remove ordinary worker finite-goal closure or
  `blocked` recovery.
- Ordinary app waiting and an explicitly scheduled follow-up are different
  surfaces. Codex MUST NOT create or recreate a heartbeat or automation as a
  workaround for an app watcher. If no supported watcher or wake route exists,
  Codex MUST record the exact limitation and bounded fallback and MUST NOT
  invent a monitor identity.

## Recovery, evaluation, and handoff

- Missing output, truncated extraction, or a wrong item-type filter does not
  prove that host evidence is absent. Codex MUST check the actual saved
  artifact, worktree, source record, and creating surface. Codex MUST NOT
  invent a checkpoint path, retry a known-absent path, or restart completed
  proof after compaction without a concrete reason.
- When evaluation of this guidance is explicitly in scope, Codex MUST use the
  existing bounded private skill-evaluation workflow. The evaluator MUST keep
  cases, holdout inputs, expected behavior, exact invocations, raw measurements,
  and complete results in evaluator-controlled private artifacts and MUST publish
  only the allowed hash/status/failure-dimension/cost summary. The ordinary
  supervision loop MUST NOT require a private campaign, answer table, fixed
  case matrix, or unmeasured savings claim.
- When evaluation is in scope, the evaluator MUST use actual Astra/medium parent
  and Luna/max worker/watcher calls where those roles matter and MUST report
  unavailable telemetry as unmeasured. The evaluator MUST choose cases and
  comparisons proportionate to the changed boundary rather than copying a
  hidden answer.
- Codex MUST finish with current branch, PR state, exact head/base, changed files, actual
  app observations, evaluation limitations, measured versus unmeasured usage,
  retained review history, unresolved findings, and one parent-owned next
  action. A passing test, callback, watcher result, open PR, or merged change is
  not a substitute for the other required evidence.
