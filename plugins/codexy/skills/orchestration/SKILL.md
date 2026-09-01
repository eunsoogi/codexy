---
name: orchestration
description: Use when classifying workflow, surface, and risk or coordinating ownership, goals, agents, threads, worktrees, reviews, compaction, and handoff; load only applicable authorities.
---

Read request/issue/PR/AGENTS.md and classify task/surface/risk. Read the
[context retention contract](references/context-tiers.md) for the existing
runtime handoff/route contract, then load only the relevant references below;
progressive disclosure does not require loading every reference. Compaction:
current wins; missing proof MUST NOT permit action.

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

- MUST read [review profiles](references/review-profiles.md) when selecting the
  single applicable reviewer and review quota.
- MUST read [review lifecycle](references/review-lifecycle.md) when waiting for,
  repairing from, or handing off a reviewer result.
- MUST read
  [plain-language user replies](references/plain-language-user-replies.md) when
  writing a user-facing progress, blocker, or completion update.
- MUST read
  [natural Korean user replies](references/natural-korean-responses.md) when the
  user-facing update is in Korean.
