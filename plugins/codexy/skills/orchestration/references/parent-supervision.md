# Orchestrator Supervision (compatibility filename: `parent-supervision.md`)

Codex MUST use this reference when an Orchestrator coordinates issue-sized work
through Codex app tasks, a Watcher, Worker callbacks, drift correction, or a
long-lived release goal. It is the canonical detailed source for delegated
supervision, evidence, and authority boundaries; it does not create a scheduler,
replace goal-lifecycle, or choose a repository's GitHub policy.

## Canonical role mapping

When this arrangement is authorized, Codex MUST keep these roles distinct:

- The Orchestrator owns goal, coordination, verification, correction, and
  acceptance; it MUST keep its exact goal active via `get_goal`. For child-owned
  lanes it MUST assign one Worker or retain an existing usable owner; it MUST
  NOT implement, patch, or duplicate the lane, or merge/publish without separate
  authority.
- The bounded, read-only native `codexy-watcher` MUST NOT own, recreate, or
  transfer task/release goals; finite goals MUST stay in scope.
- Worker MUST own files, branch, verification, review patches, and finite goal.
  Worker MUST own idle-wait and blocked recovery; Watcher MUST stay distinct.

Codex MUST auto-select model/effort; MUST NOT ask users to choose or reconfirm:

- Orchestrator and Worker-to-Orchestrator: `gpt-6-luna` / `max`.
- Worker creation MUST use [routing](child-routing-policy.md); parent-to-Worker
  follow-ups MUST preserve the authenticated recipient's pair.
- Watcher: `gpt-6-luna` / `max`; Inspector: `gpt-6.1-sol` / `medium`.

New-role thread creation and delivery MUST use the explicit model/effort pair
from the [child-routing policy](child-routing-policy.md). Authorized
blocked-goal fork continuation MUST preserve the source settings under that
policy; role defaults and sender settings MUST NOT override them.

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
- Workers MUST remain independent Codex app tasks in the same saved project. A
  separately assigned Watcher MUST use the host's callable native-subagent API
  (`spawn_agent` or its versioned equivalent) from the Orchestrator with the
  packaged `codexy-watcher` role; it is not an app task or automation. For that
  assignment, record the Worker project id, Watcher identity, and creating
  tool. A projectless task, standalone app watcher, unpackaged subagent, or
  UUID-only API is not a substitute for this subagent route.
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

## Direct observation and explicit Watcher assignments

- Ordinary Worker observation belongs to the Orchestrator, which MUST use
  `wait_threads` with exact task/host IDs and the latest cursor per target.
  Keep targets across turns; one completion or attention event MUST NOT end
  other targets or the issue. Routine progress MUST NOT use a Watcher relay.
  Read a target only for missing, truncated, contradictory, or unbound evidence.
- A Watcher route applies only to an explicitly bounded assignment. Workers
  report to that exact Watcher; its wait and non-duplication rules are defined
  below. Initial goal calls stay local; blocked/terminal receipts remain
  direct-parent.
- A verified-unavailable Watcher route or emergency permits one marked
  direct-parent fallback in that assignment; the Worker reports the limitation
  once and MUST NOT duplicate routine messages.
- Bundled `codexy-watcher` owns lifecycle criteria, stale-gate suppression,
  current-action summaries, and `watcher_report` arguments. Its emission order
  and `tools/list` descriptions MUST honor parser limits on generic hosts.
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
- At useful checkpoints or signals, the Watcher MUST inspect the smallest
  relevant artifact, diff, or call against accepted scope, ownership, and latest
  user constraints; MUST NOT rely only on status, HEAD, or self-report. MUST
  distinguish progress from material drift. Report only the latter. For the same
  operation, target, and intended outcome, later authoritative success MUST mark
  its earlier error historical; only new failure evidence can reopen it.
- A credible drift report MUST identify the concrete source or call inspected,
  the conflicting current requirement, and the consequence or remaining
  uncertainty in concise natural prose, without a fixed narrative template. This
  does not waive the exact `watcher_report` argument contract in the bundled
  Watcher role. The Watcher reports only; the Orchestrator judges and instructs
  the Worker, the Worker repairs, and the Orchestrator verifies the next
  relevant source, call, diff, or result. A later Watcher observation may report
  a new mismatch but MUST remain read-only and MUST NOT own that correction
  loop.

## Waiting and direct correction

- For ordinary Orchestrator-owned Worker waits, the owner MUST use
  cursor-based `wait_threads` with batched targets and the latest cursor for
  each target. An unchanged cursor, bounded timeout, or long command is not a
  stall; continue through the supported wait without routine status messages or
  transcript reads. A first result MUST NOT close other active targets or a
  later turn on the same task. Native reviewer delivery remains a separate
  surface, and targets assigned to a Watcher stay on its explicit route.
- Native Watcher routes: only the assigned Watcher MAY call `wait_threads`; the
  Orchestrator MUST await `watcher_wait`. This verified host's quiet route nests
  `watcher_wait(timeoutMs=295000)` in one `functions.exec` with first-line
  `// @exec: {"yield_time_ms":300000}`; two parent calls reached timeout. Hidden
  inference and other hosts remain unverified. The Orchestrator MUST omit
  `timeoutMs` only when 295,000 ms fits confirmed host/stricter limits;
  otherwise MUST pass an explicit supported value. `MAX_WAIT_MS` remains
  3,600,000 ms. Output-yield cadence MUST NOT shorten semantic waits. The
  Orchestrator MUST follow [runtime-heartbeats](runtime-heartbeats.md) and
  [token-efficient coordination](token-efficient.md) for quiet waits, early
  yields, shorter/empty waits, and fallback handling.
- Implementation Workers MUST NOT use Orchestrator-owned Watcher session/token.
  One bounded readback after an actionable report is judgement-only.
- When an actionable signal arrives, the Orchestrator MUST send one grouped,
  actionable correction to the existing worker: observed deviation, smallest
  repair, required evidence, and next permitted step. An acknowledgement is not
  proof. The Orchestrator MUST read back the next relevant actual tool call,
  diff, or result. A cooperative freeze-and-report instruction does not cancel
  an in-flight tool unless the host proves a hard stop.
- On an explicit Watcher route, fallback inspection MUST require a concrete
  signal or checkpoint. It MUST NOT become continuous transcript polling or
  direct polling of assigned targets; while they remain nonterminal, the
  Orchestrator MUST keep the same assistant turn via `watcher_wait` under host
  limits and quiet-wait rules. An empty wait result or tool output yield MUST
  NOT end the turn, narrow the objective, or trigger a final status.

## Explicit Watcher MCP flow

- The Orchestrator creates one native Watcher subagent for a bounded assignment,
  then opens one scoped `watcher_open` session for the Orchestrator, Watcher,
  and exact Worker targets. The MCP session transports observations; it does not
  create or judge the subagent.
- The Watcher MUST follow [result rules](../../../agents/codexy-watcher.toml).
  Sufficient exact target/turn results MUST skip reads/reports when no criterion
  is due. Missing, truncated, contradictory, or unbound evidence requires a
  bounded read of that target. Explicit status MUST queue `health`; required
  metadata-only updates MUST remain non-queued without new triggers or
  heartbeats.
- The Orchestrator MUST consume one `watcher_wait`: `status`, `sessionId`,
  `nextCursor`, `events`; `health.status`, `watcherState`,
  `lastObservationAtMs`, `lastError`; cancelled-only `cancellationReason`.
  Events MUST contain exactly these seven fields: `eventId`, `sequence`, `kind`,
  `target`, `summary`, `observedAtMs`, and `evidence`. MUST use assignment
  context for removed identity/transport; MUST call `watcher_health` only for
  concrete unresolved diagnostics. Its timestamp is not acceptance proof; MUST
  read Worker/app state only as needed.
- Same-connection `notifications/cancelled` or packaged Watcher
  `PreToolUse`/`Interrupt`/`UserPromptSubmit` releases only that wait when the
  host delivers the event and preserves the durable session. `watcher_cancel`
  instead ends the session and requires a fresh assignment. See
  [Watcher wait cancellation](watcher-wait-cancellation.md) for event matching,
  fail-closed cases, and host-propagation proof boundaries. A cancelled
  queue/cursor is readback only, never continuity; the assignment MUST NOT
  resume.
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
