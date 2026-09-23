# Orchestrator Supervision (compatibility filename: `parent-supervision.md`)

Codex MUST use this reference when an Orchestrator coordinates issue-sized work
through Codex app tasks, a Watcher, Worker callbacks, drift correction, or a
long-lived release goal. It is the canonical detailed source for delegated
supervision, evidence, and authority boundaries; it does not create a scheduler,
replace goal-lifecycle, or choose a repository's GitHub policy.

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

Configuration metadata MUST remain separate from role identity: Orchestrator and
Worker-to-Orchestrator delivery use `gpt-6-astra`/`medium`; Worker creation,
Orchestrator-to-Worker delivery and Watcher use `gpt-6-luna`/`max`; inspector
uses `gpt-6-sol`/`medium`. Host/runtime direction identifiers remain serialized
compatibility values; every app delivery MUST name its model and thinking
effort.

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
  unpackaged subagent, or an app API that merely accepts a UUID is not a
  substitute for this subagent route.
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

- Native Watcher assignments route each ordinary Worker report to its exact
  Watcher task. Reports MUST carry source Worker task and issue/PR lane (or an
  explicit no-PR marker); Workers MUST NOT infer targets from transcript
  visibility. The Watcher MUST validate assignment, separate tasks/lanes,
  deduplicate identities, and relay action-required deltas through
  `watcher_report`. Goal and terminal receipts remain direct-Orchestrator;
  reports are signals, not acceptance.
- A verified-unavailable route or concrete emergency permits one marked
  direct-Orchestrator fallback; Worker MUST report one limitation and MUST NOT
  resume routine direct reporting or duplicate it. Routine reads and
  liveness-only goal status MUST remain internal; the Watcher MUST NOT wake the
  Orchestrator. Only actionable lifecycle/drift, failure, missing delivery, or a
  ready gate may wake the Orchestrator.
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

- For ordinary non-Watcher app-task waits, the observation owner MUST prefer
  cursor-based `wait_threads` with batched targets. Unchanged cursors, bounded
  timeouts, and long commands are not stalls; Codex MUST NOT poll, repeat
  status, read transcripts, rerun tests, or interrupt reviewers solely for
  elapsed time. A native reviewer's terminal delivery is a non-Watcher target;
  the Orchestrator MUST NOT observe Worker targets assigned to a native Watcher.
- Native Watcher routes: only the assigned Watcher MAY call `wait_threads`; the
  Orchestrator MUST await `watcher_wait`, omitting `timeoutMs` for the 300,000
  ms default; explicit `timeoutMs=300000` is equivalent; `MAX_WAIT_MS` stays
  3,600,000 ms. The host limit MUST be read and reported: if it supports 300,000
  ms, the Watcher MUST use `wait_threads(timeoutMs=300000)` as the semantic
  event wait; otherwise, it MUST use the confirmed actual maximum. Output-yield
  cadence is separate; MUST NOT shorten or replace the semantic wait. With a
  confirmed 300-second MCP transport, `timeoutMs=295000` is the empty-wait
  margin. Shorter waits MUST have a reason and MUST NOT become repeated polls.
  While pending, the Orchestrator MUST stay in one quiet tool await and MUST NOT
  emit reasoning, progress, short polls, unrelated work, retry, or poll merely
  because no event has arrived. Fallbacks MUST report and recover.
- Implementation Workers MUST NOT use Orchestrator-owned Watcher session/token.
  One bounded readback after an actionable report is judgement-only.
- The native Watcher loop is defined in "Watcher MCP flow"; its report, Worker
  completion, empty timeout, or unchanged progress MUST NOT end the turn.
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
  Watcher observes through `watcher_wait`; its goal remains active and owned by
  the Orchestrator.

## Watcher MCP flow

- The Orchestrator creates one native Watcher subagent for a bounded assignment,
  then opens one scoped `watcher_open` session for the Orchestrator, Watcher,
  and exact Worker targets. The MCP session transports observations; it does not
  create or judge the subagent.
- The Watcher MUST use `wait_threads` with each target's latest cursor and
  inspect actual Worker results before calling `watcher_report` for a material
  event or requested health. It MUST continue its cursor loop while a target
  remains nonterminal. `watcher_health` is freshness evidence, not acceptance;
  reports are untrusted and contain no repair directive.
- The Orchestrator calls `watcher_wait` with documented `parent` capability and
  cursor; `parent` is a preserved protocol field, not a product role. It
  validates each event, reads the Worker/app surface, and sends or verifies
  corrections through the supported Worker route.
- Same-connection `notifications/cancelled` or packaged Watcher
  `PreToolUse`/`Interrupt` releases only that wait when the host propagates it
  and preserves the durable session; `watcher_cancel` is separately authorized
  and durably ends it. A host/task message or outer wait may leave it active
  when cancellation is unavailable; report that limitation, keep
  installed-candidate Stop success unproven until propagation is verified, and
  open a new assignment/session. A cancelled queue/cursor is readback only,
  never continuity; the assignment MUST NOT resume.
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
- Ordinary app waiting and scheduled follow-up are separate. Codex MUST NOT use
  a heartbeat or automation as an app Watcher workaround. If no supported
  Watcher/wake route exists, Codex MUST record the exact limitation and bounded
  fallback; it MUST NOT invent a monitor identity.

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
