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
- A Worker app task MUST own implementation files, branch, verification, review
  patches, and its finite goal. Its idle-wait and blocked recovery MUST stay
  separate from the Orchestrator's lifecycle; the Watcher MUST remain distinct.

Codex MUST auto-select model/effort; MUST NOT ask users to choose or reconfirm:

- Orchestrator and Worker-to-Orchestrator: `gpt-6-astra` / `medium`.
- Worker creation, Orchestrator-to-Worker, Watcher: `gpt-6-luna` / `max`.
- Inspector: `gpt-6.1-sol` / `medium`.

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
  `watcher_report`; blocked/terminal receipts go to the Orchestrator. Initial
  goal calls MUST stay local and unreported; reports are not acceptance.
- A verified-unavailable route or emergency permits one marked direct-parent
  fallback. Worker MUST report one limitation and MUST NOT resume or duplicate
  routine direct reports. Reads and liveness-only goal state MUST stay internal.
- Bundled `codexy-watcher` owns lifecycle criteria, stale-gate suppression,
  current-action summaries, and `watcher_report` arguments. Its ordered emission
  sequence governs reports; packaged `tools/list` bounds and descriptions MUST
  reflect parser limits, even with a generic host schema.
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
  uncertainty in concise natural prose, without a fixed narrative template. This
  does not waive the exact `watcher_report` argument contract in the bundled
  Watcher role. The Watcher reports only; the Orchestrator judges and instructs
  the Worker, the Worker repairs, and the Orchestrator verifies the next
  relevant source, call, diff, or result. A later Watcher observation may report
  a new mismatch but MUST remain read-only and MUST NOT own that correction
  loop.

## Waiting and direct correction

- For ordinary non-Watcher app-task waits, the observation owner MUST prefer
  cursor-based `wait_threads` with batched targets. Unchanged cursors, bounded
  timeouts, and long commands are not stalls; Codex MUST NOT poll, repeat
  status, read transcripts, rerun tests, or interrupt reviewers solely for
  elapsed time. A native reviewer's terminal delivery is a non-Watcher target;
  the Orchestrator MUST NOT observe Worker targets assigned to a native Watcher.
- Native Watcher routes: only the assigned Watcher MAY call `wait_threads`; the
  Orchestrator MUST await `watcher_wait`. Omit `timeoutMs` only when its 295,000
  ms default is within the confirmed host limit and every stricter applicable
  higher-priority bound; otherwise pass an explicit timeout within that bound.
  The runtime `MAX_WAIT_MS` ceiling is 3,600,000 ms. A supported shorter wait or
  its empty result alone MUST NOT alert. Report a host/fallback limit only when
  no supported route permits continuation and parent action is required.
  Output-yield cadence is separate; MUST NOT shorten semantic waits. Shorter
  waits need a reason and MUST NOT become repeated polls. While pending, the
  Orchestrator MUST stay in one quiet await and MUST NOT emit reasoning, status,
  retries, or unrelated work merely because no event arrived; use supported
  fallbacks without alerting on resolved limits.
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
  targets in a native Watcher route. While assigned targets remain nonterminal,
  the Orchestrator MUST keep the same assistant turn via `watcher_wait` under
  host limits and quiet-wait rules. An empty wait result or tool output yield
  MUST NOT end the turn, narrow the objective, or trigger a final status.

## Watcher MCP flow

- The Orchestrator creates one native Watcher subagent for a bounded assignment,
  then opens one scoped `watcher_open` session for the Orchestrator, Watcher,
  and exact Worker targets. The MCP session transports observations; it does not
  create or judge the subagent.
- The Watcher MUST use `wait_threads` per its role contract: exact assigned
  Worker targets, supplied hosts, per-target cursors, eight-target batches, and
  a `timeoutMs` within callable and higher-priority limits. If recent state is
  absent, it MAY read only that same target; recent reads omit cursors and older
  reads use that thread's page cursor. Wait/read cursors are distinct. Missing
  or truncated state remains unknown and alone MUST NOT establish current
  failure or a due parent action. It MUST wait while targets are nonterminal.
  Health-only updates are not queued; requested status uses a current-only
  `health` event. Reports stay untrusted and carry no repair directives.
- The Orchestrator calls `watcher_wait` with `sessionId`, `parentToken`, and the
  returned `cursor`; `parentToken` is the parent capability. It validates each
  event, reads the Worker/app surface, and sends or verifies corrections through
  the supported Worker route.
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
