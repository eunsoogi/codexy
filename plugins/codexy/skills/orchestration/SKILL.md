---
name: orchestration
description: MUST use first for Codexy task classification and then for goals, plans, issue-sized lanes, threads, worktrees, compaction ledgers, token discipline, and orchestrator-led execution loops.
---

# Orchestration

## Purpose

MUST run the current plugin-invoking Codex thread as the root/orchestrator for
goal-oriented work. MUST NOT spawn or assign a separate orchestrator agent. The
invoking Codex thread owns intent, decomposition, routing, evidence integration,
and final completion claims. Specialists and separate Codex thread/worktree
lanes own bounded atomic units only; Root `AGENTS.md` owns repo-wide dogfooding
policy.

## Read Next

MUST read these relative references before acting on the matching surface:

- `references/task-classification.md` and
  `references/classification-and-control.md` for classification, goal, plan,
  child execution, multi-agent, codegraph, LSP, and Sentinel discipline.
- `references/goal-transition-reporting.md` for delegated parent goal-report
  receipts.
- `references/plugin-public-contracts.md` when an installed extension invokes
  `$orchestration` across the Codex plugin boundary.
- `references/thread-and-worktree-routing.md` for parent/child boundaries,
  thread discovery, Codex app worktree preflights, and worktree rules.
- `references/orchestration-loop.md` for intake, plan, dispatch, integration,
  verification, finish, failure modes, and handoffs.
- `references/runtime-heartbeats.md` for external waits.
- `references/parent-stop-preflight.md` for ownership checks before
  implementation edits.
- `references/execution-budget.md` for finite child execution and termination.
- `references/token-efficient.md` for compact event deltas and token discipline.
- `references/plain-language-user-replies.md` and
  `references/natural-korean-responses.md` for user-facing replies and separate
  machine-readable evidence.
- `references/child-routing-policy.json`,
  `references/routing-evaluation-corpus.json`,
  `references/routing-evaluation-results.schema.json`, and
  `references/routing-evaluation-results.json` for structured child-routing
  selection and frozen paired measurement.
- `references/review-profiles.json` and
  `references/workflow-review-classification.json` for structured review budgets
  and exhaustive typed profile selection.
- `references/review-lifecycle.md` for profile-selected multi-agent terminal
  review, post-cap completion, and remaining proof gates.

## Classification Gate

MUST classify the lane through this skill before setup, validation, release,
delegation, implementation, PR handling, review-response routing, or merge
coordination for Codexy work. Classification evidence MUST name the lane type,
owner decision, atomic scope, required skills, required tools or evidence, first
allowed action, and any stop blocker. Missing classification before setup,
validation, release, or other workflow actions is a workflow defect: MUST stop,
classify, and only then MUST continue through the matching Codexy workflow.

## Authority Boundary

`references/task-classification.md` is the authoritative ownership contract; its
formal classification gate MUST run before setup or action.

## Packaged Agents

MUST read `references/agent-registration.md` before registering, updating,
uninstalling, diagnosing, or invoking a packaged specialist.

## Required Control Plane

- MUST establish the goal before implementation. If `create_goal` is available,
  MUST use it directly for non-trivial delegated or orchestrated lanes; MUST use
  `get_goal` to inspect active goal state when needed; MUST use `update_goal`
  only when completion or true blockage is proved.
- MUST maintain a visible todo list with real `update_plan` or todo-tool state
  for any non-trivial task when available. Prose-only todo text is insufficient
  unless the todo/plan tool is unavailable and the fallback is reported.
- MUST treat asynchronous completion as event waits, not blockers. Parent
  orchestrators and child owners MUST use event-driven `wait_threads` with each
  target's latest cursor as the default for ordinary child completion or
  attention waits. They MUST reserve heartbeat scheduling for genuinely
  scheduled monitoring or when `wait_threads` is unavailable. After a host
  transition or `No handler registered` failure, the owner MUST treat the
  mismatch as host-transition exposure evidence, perform one fresh thread-tool
  discovery and one host-aware `wait_threads` retry before any fallback, MUST
  NOT use unbounded `read_thread`, and any bounded metadata fallback MUST
  consume the current parent-stage budget and record only returned size/token
  metadata. When an eligible external gate outlives the turn, they MUST follow
  `references/runtime-heartbeats.md`. Live Sentinel observation MUST be
  read-only and event-driven. Generic child and ledger polling remains
  permitted. Both the child owner and the root orchestrator MUST NOT message,
  interrupt, replace, duplicate, follow up with, or poll a live Sentinel. A
  bounded wait with no event is a non-terminal `PENDING` observation, and an
  independently observed live reviewer is `RUNNING`; neither observation is a
  reviewer verdict or fallback-eligible. The owning lane MUST retain the same
  reviewer and wait for its natural terminal result. A live Sentinel MUST report
  its own terminal `PASS`, `BLOCK`, or `UNOBSERVABLE` result naturally.
- In long multi-issue or multi-PR polling loops, MUST preserve all proof gates
  while carrying only current deltas.
- Opening a PR is not completion when the requested outcome includes completion,
  merge, default Codexy merge flow, or no explicit stop/wait/
  draft-only/leave-open instruction.
- If a completion or handoff artifact reports completion while a matching clean
  PR remains open, validate it with
  `scripts/validate-plugin-config.sh --check-completion-handoff --handoff-file <report> --pr-state-file <gh-pr-view-json>`.
  If the report discusses addressed review feedback, the PR state evidence MUST
  include GraphQL `reviewThreads.nodes`.

## Delegation, Review, And Handoff

MUST use the matching references for active-child ledgers, worktree
reservations, specialist selection, profile-selected review, codegraph/LSP
availability, and compact event-driven handoffs. The root orchestrator owns
planning and parent integration; a child owns only its assigned atomic lane.
Subagents are helpers, never worktree owners. Every delegated helper MUST NOT
spawn, delegate to, or create another agent, task, or thread.

MUST follow `references/review-lifecycle.md` for profile-selected multi-agent
review selection, terminal verdict accounting, post-cap completion, repair, and
proof gates. A finding MAY block only when it maps to the issue contract or a
root correctness, safety, or readiness defect; adjacent edge cases, syntax
variants, and speculative hardening are non-blocking follow-up candidates and
MUST use approved issue intake if tracked.

MUST follow `references/parent-stop-preflight.md` before implementation edits,
including its child-lane ownership validation when required. `blocked` is only
for an unanswered material user decision, never an asynchronous wait.
