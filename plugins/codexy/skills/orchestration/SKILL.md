---
name: orchestration
description: Use when classifying workflow, surface, and risk or coordinating ownership, goals, agents, threads, worktrees, reviews, compaction, and handoff; load only applicable authorities.
---

Read request/issue/PR/AGENTS.md and classify task/surface/risk. Read the
[context retention contract](references/context-tiers.md) for the existing
runtime handoff/route contract, then load the references selected by that route;
these are the canonical handoff references. When a separate condition requires
additional guidance (for example, app-thread workers, a watcher, or delegated
goal ownership), the agent MUST load that reference when the condition applies
and MUST NOT add it to unrelated routes. Progressive disclosure does not require
loading every reference. A GitHub surface alone does not select review-specific
references. Compaction: current state MUST win. Missing proof MUST NOT authorize
a completion/readiness claim or the specific action whose effect requires that
proof; it MAY require evidence collection or leave authorized reversible work in
progress, but it MUST NOT by itself require a new approval to continue that
work.

Verification procedure remains owned by the selected references:
[workflow profiles](references/workflow-profiles.md) selects the profile,
[TDD classification policy](references/tdd-classification-policy.md) selects
per-boundary sequencing and proof,
[execution budget](references/execution-budget.md) bounds finite work and
evidence reuse, [review profiles](references/review-profiles.md) and
[review lifecycle](references/review-lifecycle.md) own current-head review, and
[proof-driven completion](../proof-driven-completion/SKILL.md) owns final
claims. Do not restate these policies here or preload them for an unrelated
route.

### Readable communication boundary

All assistant-authored text that a person may see MUST follow the shared
[plain-language message rule](references/plain-language-user-replies.md). This
includes parent replies, task/agent prompts, delegated instructions, progress
and callback messages, handoffs, and tool prompt fields. Concision and protected
technical text MUST follow that contract.

### Delegated supervision route

When a task uses app-thread Workers, a native Watcher, Worker callbacks, drift
correction, or delegated Worker goal ownership, MUST read
[parent supervision](references/parent-supervision.md) before dispatch,
observation, reporting, interruption, correction, or handoff. That reference is
the canonical source for role ownership, assigned model policy, Worker report
routing, Watcher observation and waits, deduplication, host limits, interruption
and cancellation, drift correction, and supervision evaluation.

The route has these mandatory boundaries: the Orchestrator owns the overall
goal, judgement, correction, and acceptance; the Worker owns its implementation
and finite goal; the Watcher is read-only and observes only its assigned
Workers; only the assigned Watcher MAY call `wait_threads` for those Worker
targets; and the Orchestrator MUST await `watcher_wait` and MUST NOT wait on
those targets directly. Workers MUST NOT use an Orchestrator-owned Watcher
session or token. A Watcher report or finite goal MUST NOT transfer ownership or
prove issue completion. A native reviewer's own terminal delivery follows its
review route and MUST NOT be treated as Worker observation or permission for
direct Orchestrator polling. Read `parent-supervision.md` for exact wait values,
host-limit, quiet-wait, fallback, report, interruption, cancellation, and
non-Watcher conditions before execution.

### Planning and execution boundary

`$planning` is the source of instructions for plan content, plan updates, and
local plan-file rules. MUST select it for an explicit plan-creation or
plan-update request, or when genuinely necessary decomposition is required for
large work. Orchestration remains authoritative for task classification,
ownership, assignment, worktree, dispatch, execution coordination, and merge. A
plan is a supporting artifact: it MUST NOT create execution authority, replace a
native goal, or prove that execution or verification occurred.

A plan-only request MUST return the planning result and MUST NOT create an
issue, branch, worktree, child task, or implementation. When execution is
already authorized for one atomic issue, orchestration MUST NOT require a plan
file or a new approval for ordinary implementation choices, and MUST NOT
re-enter planning when it is already selected and in progress.

### Permission boundary

- Before asking for approval, MUST identify the next action and test whether it
  changes scope, target, risk, authority, or external state. MUST complete
  independent authorized preparation first, so any request is tied to a
  concrete, reviewable result. Ordinary execution, implementation choices,
  read-only investigation, evidence collection, reviewable preparation, and
  in-scope reversible work MUST NOT require a new approval when the current
  request, its reasonable implied scope, or prior authorization covers them.
- Missing proof, an unavailable diagnostic, uncertainty about an implementation
  detail, a parent correction, or a changed internal record MUST NOT by itself
  become a permission gate. It MAY require more evidence or MAY withhold a
  completion/readiness claim.
- Quoted, historical, or negated text MUST be classified by its source and
  operative scope: a quoted prior statement or example MUST NOT be treated as a
  new user decision, while an explicit current-user prohibition MUST remain
  binding.
- MUST ask only when the next action has a concrete material user decision or
  requires authority not already provided, including external/destructive
  authority. The request MUST name the precise decision or missing authority and
  why it is required. The agent MUST NOT infer issue authorization for every
  merge, publication, external message, or destructive action.
- When host, credential, branch, or environment authority is denied or
  unavailable, the agent MUST report the exact constraint and MUST identify the
  action or owner needed to resolve it; a repeated approval question MUST NOT
  substitute for missing access or a protection rule.

### Classify and route

- MUST read [task classification](references/task-classification.md) when
  recording ownership and the atomic lane.
- MUST read [workflow profiles](references/workflow-profiles.md) when choosing
  the proportionate light, standard, or strict profile.
- MUST read [TDD classification policy](references/tdd-classification-policy.md)
  when deciding between engineering tests and proportional proof.
- MUST read [child-routing policy](references/child-routing-policy.md) when
  selecting a packaged specialist or generic child route.
- MUST read
  [thread and worktree routing](references/thread-and-worktree-routing.md)
  before creating, reusing, or recycling a child thread or worktree.
- MUST read
  [classification and control](references/classification-and-control.md) when
  assigning ownership, coordinating a child, or applying stop gates.
- MUST read [parent supervision](references/parent-supervision.md) when an issue
  uses app-thread workers, a watcher, worker callbacks, drift correction, or
  delegated goal ownership.
- MUST read [agent registration](references/agent-registration.md) when
  discovering or invoking packaged specialist agents.

### Execute and report

- MUST read [orchestration loop](references/orchestration-loop.md) when moving
  through planning, delegation, verification, or handoff.
- MUST read [execution budget](references/execution-budget.md) when capping
  repair, review, fanout, or wait work.
- MUST read [token-efficient coordination](references/token-efficient.md) when
  recovering context, polling, or preparing a compact handoff.
- MUST read [runtime heartbeats](references/runtime-heartbeats.md) when waiting
  for child events or deciding whether scheduled monitoring applies.
- MUST read [goal transition reporting](references/goal-transition-reporting.md)
  when delivering child goal state or a terminal transition to the parent.
- MUST read [parent stop preflight](references/parent-stop-preflight.md) before
  an implementation edit that may need child-owned Git or PR state.
- MUST read [plugin public contracts](references/plugin-public-contracts.md)
  when a plugin, connector, or installed-surface contract is involved.

### Review and communicate

- MUST read [review profiles](references/review-profiles.md) when selecting a
  proportionate current-head reviewer.
- MUST read [review lifecycle](references/review-lifecycle.md) when waiting for,
  repairing from, or handing off a current-head reviewer result.
- MUST read
  [plain-language user replies](references/plain-language-user-replies.md) when
  writing any user-visible message, including task/agent prompts, callbacks,
  handoffs, and tool prompt fields.
- MUST read
  [natural Korean user replies](references/natural-korean-responses.md) when the
  user-visible message is in Korean.
