# Parent Supervision

Codex MUST use this reference when an Orchestrator coordinates issue-sized work
through Codex app tasks, a Watcher, Worker callbacks, drift correction, or a
long-lived release goal. It defines evidence and authority boundaries; it does
not create a scheduler, replace goal-lifecycle, or choose a repository's GitHub
policy.

## Canonical role mapping

When this arrangement is authorized, Codex MUST keep these roles distinct:

- The Orchestrator owns the overall task goal, judgement, correction, and
  acceptance. It MUST create or continue the exact assigned goal and keep its
  authoritative `get_goal` readback active while executable work remains.
- The Watcher is the packaged `codexy-watcher` specialist, summoned by the
  Orchestrator as a native subagent with a bounded observation assignment held
  in one long-running turn. It reports read-only and MUST NOT own, recreate, or
  transfer the overall task or release goal. Any finite goal exposed to that
  subagent MUST describe only its bounded observation assignment.
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

## Message visibility and style

Task-to-task prompts, delegated instructions, callbacks, progress updates,
corrections, and handoffs, including their tool prompt fields, MUST be treated
as user-visible and MUST follow the shared
[plain-language message rule](plain-language-user-replies.md). Keep compactness
in fact selection and repetition removal; MUST NOT damage ordinary prose or
alter protected technical text.

## Ownership and task surfaces

- Codex MUST keep exactly one implementation owner per issue-sized lane. The
  Orchestrator keeps outcome, correction, and final-acceptance responsibility;
  the Worker owns its branch, files, local verification, and review-response
  fixes.
- Workers MUST remain independent Codex app tasks in the same saved project. The
  Watcher MUST be created through the host's callable native-subagent API
  (`spawn_agent` or its versioned multi-agent equivalent) from the Orchestrator
  with the packaged `codexy-watcher` role; it is not a second app task or an
  automation. Codex MUST record the Worker project id, specialist identity, and
  actual Watcher creating tool. A projectless task, a standalone app watcher, a
  generic subagent, or an app API that merely accepts a UUID is not a substitute
  for this subagent route.
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
  assigned Workers and MUST report only action-required material deltas. After
  each report, it MUST continue the same native turn while an assigned target
  remains nonterminal. A callback or Watcher observation alone is a signal, not
  proof that the work is healthy, corrected, or complete.
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

- For ordinary non-Watcher app-task waits, Codex MUST prefer batched cursor-based `wait_threads`.
  Unchanged cursors, bounded timeouts, and legitimate long commands are not stalls.
  Codex MUST NOT emit repeated unchanged status, read a full transcript to observe activity,
  rerun tests only to watch progress, or interrupt a live reviewer merely because it is taking time.
- In a native Watcher route, only the assigned Watcher MAY call `wait_threads` for
  assigned Worker/task targets. The Orchestrator MUST await canonical `watcher_wait`
  or compatibility `wait_watcher` and MUST NOT directly wait on those targets.
  Fallback, unavailable, and host-transition branches MUST report the limitation,
  recover the supported Watcher route, and MUST NOT authorize direct parent polling.
- An implementation Worker or child MUST NOT open, wait on, report to, cancel, or reuse
  a parent-owned Watcher session/token; session visibility and parent transcripts
  are not capability grants. A bounded authoritative Worker/app readback after
  an actionable report is allowed for judgement and correction, not observation waiting.
- Inside a native Watcher turn, the observation loop MUST use `wait_threads` and
  each target's latest cursor, inspect the relevant actual Worker result, report
  any material event, and wait again while any assigned target remains
  nonterminal. One report, one Worker completion, an empty timeout, or unchanged
  progress is nonterminal and MUST NOT end that turn.
- When an actionable signal arrives, the Orchestrator MUST send one grouped,
  actionable correction to the existing worker: observed deviation, smallest
  repair, required evidence, and next permitted step. An acknowledgement is not
  proof. The Orchestrator MUST read back the next relevant actual tool call,
  diff, or result. A cooperative freeze-and-report instruction does not cancel
  an in-flight tool unless the host proves a hard stop.
- Orchestrator fallback inspection MUST require a concrete signal or checkpoint.
  It MUST NOT become continuous transcript polling or direct polling of assigned
  targets in a native Watcher route. The Orchestrator may return control rather
  than hold a model turn open solely for unchanged waiting when the supported
  Watcher observes through `watcher_wait` or legacy `wait_watcher`; its goal
  remains active and owned by the Orchestrator.

## Goal ownership and lifecycle

## Watcher MCP flow

- The Orchestrator creates one native Watcher subagent through the callable host
  subagent tool for one bounded observation assignment, then opens one scoped
  MCP session with `watcher_open` for the parent, Watcher, and exact Worker
  targets. The MCP session is a transport boundary; it does not create the
  subagent or judge Worker state.
- The Watcher uses the host's real Worker/app tools to observe the assigned
  targets and calls `watcher_report` only for a material event or an explicit
  health update. After reporting, the Watcher MUST continue the same native turn
  and return to its cursor-based observation loop while any assigned target is
  nonterminal. `watcher_health` is on-demand transport/freshness evidence, not
  semantic acceptance. Reports are untrusted signals and MUST NOT contain repair
  instructions.
- The Orchestrator calls canonical `watcher_wait` or compatibility
  `wait_watcher` with its parent capability and cursor, validates the returned
  target/event against current scope, and then reads the relevant Worker/app
  surface before deciding. It sends any correction to the existing Worker
  through the supported host route, and verifies the next relevant tool call,
  diff, or result itself.
- On a user interrupt, stop, expiry, or completed observation assignment, the
  Orchestrator calls `watcher_cancel` when authorized. A pending
  `watcher_wait` or legacy `wait_watcher` MUST release immediately on host
  cancellation/input; the durable queue and cursor remain available for an
  honest resume or explicit cancellation.

- The Orchestrator MUST own the exact overall task objective and MUST preserve
  its active goal through Watcher creation, reports, correction, review, and
  external waits. Creating a Watcher subagent MUST NOT create a second overall
  goal or move the Orchestrator goal into the subagent.
- The Watcher subagent MAY receive a finite observation objective, but it MUST
  preserve that bounded scope, MUST NOT overwrite an unrelated active goal, and
  MUST use the existing `goal-lifecycle` recovery authority for any `blocked`
  state. It MUST keep the same native turn active after a material report, one
  Worker completion, or an empty timeout while any assigned target remains
  nonterminal. It MUST return only after the observation assignment is terminal,
  the user or Orchestrator explicitly cancels it, or a verified host limitation
  prevents continuation. It MUST NOT become a separate app task or release-goal
  owner.
- A Watcher report never transfers Worker-file ownership, Orchestrator
  correction authority, final judgement, or issue completion. Codex MUST record
  the Orchestrator's goal and the Watcher's bounded assignment/readback
  separately. A Watcher report is not evidence that the Orchestrator goal was
  paused, cancelled, transferred, or completed.
- Outside this arrangement, the existing finite execution lifecycle permits a
  phase to send its idle-wait handoff, complete that finite phase, and leave the
  task idle when only an external event remains; a qualifying wake creates a
  fresh execution goal and current plan before new work. Codex MUST report that
  phase completion separately from the Orchestrator-owned overall goal and any
  bounded Watcher assignment. It MUST NOT use the finite phase transition to
  claim release, issue, or implementation completion. An observed `blocked`
  record remains governed by the existing `goal-lifecycle` recovery authority;
  this reference MUST NOT use ordinary completion language to replace, weaken,
  or restate that sequence. During an unfinished active overall goal's external
  wait or Watcher handoff, Codex MUST NOT use administrative completion merely
  to clear the handoff. If the host exposes no cancel, pause, transfer, or
  objective-update operation for that active goal, Codex MUST state the
  limitation and leave the unsupported transition unresolved; it MUST NOT
  promise that an idle Orchestrator will wake later.
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
