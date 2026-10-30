---
name: orchestration
description: Use when classifying workflow, surface, and risk or coordinating ownership, goals, agents, threads, worktrees, reviews, compaction, and handoff; load only applicable authorities.
---

Read the user request, parent assignment, and applicable AGENTS.md instruction
files first. Determine whether they explicitly assign bounded work, including
read-only diagnosis or preflight. For an authorized Orchestrator or Worker task,
Codex MUST follow the required first transition in
[goal-lifecycle](../goal-lifecycle/SKILL.md) before task-specific work. That
skill owns host goal reads, native state meanings, transition rules, and
recovery; this entrypoint MUST NOT restate those rules.

After the goal gate, read issue/PR material and classify task/surface/risk. Read
the [context retention contract](references/context-tiers.md) for the existing
runtime handoff/route contract, then load the references selected by that route;
these are the canonical handoff references. When a separate condition requires
additional guidance (for example, app-thread workers or delegated goal
ownership), the agent MUST load that reference when the condition applies and
MUST NOT add it to unrelated routes. Progressive disclosure does not require
loading every reference. A GitHub surface alone does not select review-specific
references. Compaction: current state MUST win. The
[proof-driven-completion](../proof-driven-completion/SKILL.md) contract owns
evidence-to-outcome judgment; missing proof MUST withhold only claims or actions
that depend on it and MUST NOT block unrelated authorized reversible work.

When a non-routine judgment needs an Advisor's second opinion, Orchestrators and
Workers MUST follow the
[Advisor consultation contract](references/advisor-consultation.md). A Worker
MUST keep advice within its assigned issue lane and route cross-lane design,
ownership, shared-interface, or contradictory-verification decisions to the
Orchestrator. Routine actions with a clear governing rule do not need
consultation.

Verification procedure remains owned by the selected references:
[workflow profiles](references/workflow-profiles.md) selects the profile,
[TDD classification policy](references/tdd-classification-policy.md) selects
per-boundary sequencing and proof,
[execution budget](references/execution-budget.md) bounds finite work and
evidence reuse, [review profiles](references/review-profiles.md) and
[review lifecycle](references/review-lifecycle.md) own current-head review, and
[proof-driven completion](../proof-driven-completion/SKILL.md) owns final
claims. This entrypoint MUST NOT restate these policies or preload them for an
unrelated route.

### Readable communication boundary

All assistant-authored text that a person may see MUST follow the shared
[plain-language message rule](references/plain-language-user-replies.md). This
includes parent replies, task/agent prompts, delegated instructions, progress
and callback messages, handoffs, and tool prompt fields. Concision and protected
technical text MUST follow that contract.

### Task kind and owner decisions

[Task classification](references/task-classification.md#orchestration-request-ownership)
is the single decision source for request kind, scoped authority, implementation
owner, and dependency/resource/integration distinctions. MUST apply it before
route selection or work ordering. This entrypoint MUST NOT add a competing
decision; [child routing](references/child-routing-policy.md) owns dispatch
mechanics.

### Delegated supervision route

When a task uses app-thread Workers, callbacks, delegated Worker goals, drift
correction, or Worker elevation, MUST read
[parent supervision](references/parent-supervision.md) before dispatch,
observation, reporting, correction, or handoff. Eligible elevation MUST also
read [reconciliation](references/worker-elevation/reconciliation.md). These
references define role, model, report and wait routing, plus deduplication,
capacity, interruption, cancellation, drift, and evaluation.

The task-classification contract selects the owner and classifies dependencies,
resource reservations, and integration conflicts. After that decision, MUST
apply the parent-supervision and child-routing procedures without changing the
owner or repeating the task-kind decision here.

### Planning and execution boundary

`$planning` is the source of instructions for plan content, plan updates, and
local plan-file rules. MUST select it for an explicit plan-creation or
plan-update request, or when genuinely necessary decomposition is required for
large work. MUST follow
[task classification](references/task-classification.md#orchestration-request-ownership)
for plan-only versus executable work and its owner. A plan is a supporting
artifact: it MUST NOT create execution authority, replace a native goal, or
prove that execution or verification occurred.

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
- Parent instructions, direct user messages, and child self-reports MUST retain
  source and scope. A parent report MUST NOT be presented as a direct-user
  decision without its message or an authenticated reference; child self-reports
  MUST NOT be treated as host/runtime proof. Quoted, historical, or negated text
  MUST NOT become a new decision; current-user prohibitions MUST remain binding.
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

- MUST use [adaptive work](references/adaptive-work.md) when choosing execution,
  search, or verification strategy; select only the steps useful for the task.
- For repeated failure, uncertain premises, conflicting verification, or a
  meaningful strategy comparison, use
  [self-improvement](../self-improvement/SKILL.md) to select a bounded
  improvement route. Simple questions and small well-understood edits MUST keep
  the ordinary execution/check path; this route does not change ownership or
  authority.
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
  uses app-thread workers, worker callbacks, drift correction, or delegated goal
  ownership.
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
  for Worker events, handling an ordinary Worker idle-wait handoff, or deciding
  whether scheduled monitoring applies.
- MUST read [goal transition reporting](references/goal-transition-reporting.md)
  when delivering blocked-goal recovery or terminal handoff receipts to the
  parent.
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
