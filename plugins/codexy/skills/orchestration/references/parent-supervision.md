# Orchestrator Supervision (compatibility filename: `parent-supervision.md`)

Codex MUST use this reference when an Orchestrator coordinates issue-sized work
through Codex app tasks, a Watcher, Worker callbacks, drift correction, or a
long-lived release goal. It is the canonical detailed source for delegated
supervision, evidence, and authority boundaries; it does not create a scheduler,
replace goal-lifecycle, or choose a repository's GitHub policy.

Codexy packages no Watcher specialist, MCP, or runtime. Watcher flows require an
externally supplied, explicitly assigned compatible transport and entrypoint;
ordinary Worker observation remains direct.

For a general Worker whose hard-work evidence permits elevation, the parent MUST
use the [Worker elevation reconciliation](worker-elevation/reconciliation.md)
before transferring ownership. It connects the decision and #1447 transition to
these existing owner, reservation, event, and per-task cursor records; this
reference remains authoritative for shared capacity and ordinary observation.

## Canonical role mapping

When this arrangement is authorized, Codex MUST keep these roles distinct:

- The Orchestrator owns goal, coordination, verification, correction, and
  acceptance; it MUST keep its exact goal active via `get_goal`. For child-owned
  lanes it MUST assign one Worker or retain an existing usable owner; it MUST
  NOT implement, patch, or duplicate the lane, or merge/publish without separate
  authority.
- A bounded, read-only Watcher supplied outside the packaged catalog MUST NOT
  own, recreate, or transfer task/release goals; finite goals MUST stay in
  scope.
- Worker MUST own files, branch, verification, review patches, and finite goal.
  Worker MUST own idle-wait and blocked recovery; Watcher MUST stay distinct.

Codex MUST auto-select model/effort; MUST NOT ask users to choose or reconfirm:

- Orchestrator and Worker-to-Orchestrator: `gpt-6.1-sol` / `low` (Sol / Light).
- Worker creation MUST use [routing](child-routing-policy.md); parent-to-Worker
  follow-ups MUST preserve the authenticated recipient's pair.
- Inspector: `gpt-6.1-sol` / `medium`.

New-role thread creation and delivery MUST use the explicit model/effort pair
from the [child-routing policy](child-routing-policy.md). Authorized
blocked-goal fork continuation MUST preserve the source settings under that
policy; role defaults and sender settings MUST NOT override them.

## Message visibility and style

The [scoped assignment authority](child-routing-policy.md) covers necessary task
creation and assignment, follow-up, reporting, and recovery messages. Codex MUST
NOT ask for approval again per child or message, or treat it as host permission.

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
- Workers MUST remain independent Codex app tasks in the same saved project. If
  an external Watcher is separately assigned, record its exact host-exposed
  identity, creating tool, and Worker project id; a projectless task, standalone
  app watcher, or UUID-only API does not prove native-subagent identity.
- The Watcher MUST remain read-only observation. It MAY report a material
  failure, drift, contradiction, scope expansion, missing callback, or
  unavailable channel. It MUST NOT edit worker files, direct or message a Worker
  to change course, supply a repair directive, correct a Worker, decide
  acceptance, verify a correction, replace a Worker, or recruit another Watcher.
- Codex MUST distinguish app workers and app watchers from native subagents and
  packaged reviewers at the actual creating and readback tools. Codex MUST
  preserve native BLOCK, PASS, UNOBSERVABLE, and closed-handle history and MUST
  NOT fabricate a verdict or reset a review count to fit a newer surface.

## Supervised app-child capacity

Before creating or resuming a supervised app child, the Orchestrator MUST
reconcile its retained owner/slot ledger with actual task and setup readbacks.
The ceiling MUST be `min(8, any explicitly lower host limit, resource limit)`;
an unspecified lower limit MUST NOT be invented. Reserved, setting-up, and
active children MUST count together, once per child, across all wait batches and
turns. Moving a reservation to setup or active MUST NOT allocate a second slot.
This ceiling applies to supervised app children, not unrelated app tasks or
native `spawn_agent` concurrency. A lower helper budget still applies.

The Orchestrator MUST reserve capacity before dispatch. It MUST start ready
same-wave work concurrently without scanning paths or gating on path overlap;
actual prerequisites, permissions, and shared-resource reservations still bind.
Real integration conflicts MUST return to the owning branch before merge and
preserve both changes. At capacity it MUST leave excess dependency-ready work in
its own backlog without creating a child thread or activating a child goal. It
MUST NOT run sixteen children by dividing them into two eight-target waits. If
already above the ceiling, it MUST stop new dispatch and preserve existing
owners rather than cancel or replace them.

Pending setup identities MUST retain their reservations until authoritative
ready or failed setup evidence is reconciled. A timeout, omitted list entry,
attention event, or idle child MUST NOT release a slot. A surfaced thread MUST
replace its pending identity in the same slot. A confirmed setup failure with no
live owner MAY release that slot; an ambiguous failure MUST retain it. A bounded
not-surfaced search alone MUST NOT prove capacity was returned.

Before reuse, the Orchestrator MUST confirm completion or explicit stop/release
and no remaining live owner for the released execution. It MUST retain the
issue-owner record and latest cursor separately from capacity. Resuming that
owner for later work MUST reserve a slot first; it MUST NOT create a duplicate
owner. A newly freed slot MAY admit the next eligible backlog item only after
reconciling setup and duplicate-owner evidence.

## Direct observation and explicit Watcher assignments

- Ordinary Worker observation belongs to the Orchestrator, which MUST use
  `wait_threads` with exact task/host IDs and the latest cursor per target. Keep
  targets and the latest per-target cursor across completion, attention,
  timeout, and later turns. For every returned target, MUST save its returned
  cursor and pass it as `afterCursor` on the next wait; omitted targets MUST
  retain their prior cursor. One returned event MUST NOT remove other targets,
  reset their cursors, or prove the issue complete. A completed target's cursor
  MUST remain available for its later turn. Routine progress MUST NOT use a
  Watcher relay. Read a target only for missing, truncated, contradictory, or
  unbound evidence.
- A Watcher route applies only to an explicitly bounded assignment. Workers
  report to that exact Watcher; its wait and non-duplication rules are defined
  in [external Watcher MCP flow](external-watcher-mcp-flow.md). Initial goal
  calls stay local; blocked/terminal receipts remain direct-parent.
- A verified-unavailable Watcher route or emergency permits one marked
  direct-parent fallback in that assignment; the Worker reports the limitation
  once and MUST NOT duplicate routine messages.
- A separately supplied Watcher owns only its assigned lifecycle criteria,
  stale-gate suppression, current-action summaries, and `watcher_report`
  arguments. Its emission order and `tools/list` descriptions MUST honor parser
  limits on generic hosts.
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

- A new child blocked report MUST follow the
  [Advisor consultation procedure](advisor-consultation.md#a-new-child-blocked-report)
  before disposition, without waiting for three failures. The Orchestrator MUST
  distinguish blocked prose, native goal state, reviewer `BLOCK`, and dependency
  waits using authoritative evidence. It MUST deduplicate unchanged reports,
  preserve ownership and capacity, evaluate the read-only advice, and send an
  authorized concrete next action, verification, allowed phase, and stop
  condition to the existing child when possible. It MUST confirm the child's
  actual next call, diff, or result before claiming recovery. Native blocked
  recovery and terminal handoff MUST retain their existing lifecycle and
  [reporting](goal-transition-reporting.md) contracts; advice MUST NOT supply
  missing permission, bypass user decisions, or invent a goal transition.
- For ordinary Orchestrator-owned Worker waits, the owner MUST use cursor-based
  `wait_threads` with targets within the supervised-child ceiling and the latest
  cursor for each target. Multiple batches MUST NOT bypass that ceiling. An
  unchanged cursor, bounded timeout, or long command is not a stall; continue
  through the supported wait without routine status messages or transcript
  reads. A first result MUST NOT close other active targets or a later turn on
  the same task. Native reviewer delivery remains a separate surface, and
  targets assigned to a Watcher stay on its explicit route.
- External Watcher routes: only the assigned Watcher MAY call `wait_threads`;
  the Orchestrator MUST await `watcher_wait`. This verified host's quiet route
  nests `watcher_wait(timeoutMs=295000)` in one `functions.exec` with first-line
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
  actionable correction to the existing Worker naming the observed deviation and
  either the unmet issue criterion or a verified ownership, safety, or
  dependency constraint, plus the evidence gap. It MUST state required evidence
  and the next permitted step. The Worker MUST choose and implement any in-scope
  repair under
  [issue-owned implementation decisions](task-classification.md#issue-owned-implementation-decisions).
  An acknowledgement is not proof. The Orchestrator MUST read back the next
  relevant actual tool call, diff, or result. A cooperative freeze-and-report
  instruction does not cancel an in-flight tool unless the host proves a hard
  stop.
- On an explicit Watcher route, fallback inspection MUST require a concrete
  signal or checkpoint. It MUST NOT become continuous transcript polling or
  direct polling of assigned targets; while they remain nonterminal, the
  Orchestrator MUST keep the same assistant turn via `watcher_wait` under host
  limits and quiet-wait rules. An empty wait result or tool output yield MUST
  NOT end the turn, narrow the objective, or trigger a final status.

## Recovery, evaluation, and handoff

Read the
[recovery, evaluation, and handoff
reference](parent-supervision/recovery-evaluation-and-handoff.md) for cleanup,
evaluation, and terminal-report requirements.

The parent Orchestrator MUST perform accepted-child eligibility checks,
preservation, archival, safe local branch deletion, and outcome readback under
the
[cleanup policy](../../../../codexy-github/skills/git-workflow/references/local-git-and-branches.md#accepted-child-cleanup).
A user reminder MUST resume that flow; forwarding it does not establish
progress.
