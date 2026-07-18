---
name: codex-orchestration
description: MUST use when coordinating Codex plugin calls, long-running goals, issue-sized decomposition, multi-agent or multi-thread execution, worktrees, todo/update_plan tracking, and orchestrator-led implementation loops.
---

# Codex Orchestration

## Purpose

MUST run the current plugin-invoking Codex thread as the root/orchestrator for
goal-oriented work. MUST NOT spawn or assign a separate orchestrator agent. The
invoking Codex thread owns intent, decomposition, routing, evidence integration,
and final completion claims. Specialists and separate Codex thread/worktree lanes
own bounded atomic units only.

Root `AGENTS.md` owns repo-wide dogfooding policy. This skill supplies the
execution loop and MUST be read with root `AGENTS.md`.

## GPT-5.6 Routing Matrix

- Root/orchestrator: MUST use `gpt-5.6-sol` for decomposition, risk decisions,
  integration, and completion.
- Generic implementation, debugging, integration, and QA child thread: MUST
  explicitly request `model: "gpt-5.6-terra"` and `reasoning_effort: "high"`.
- `gpt-5.6-luna` is only for repository discovery, cataloging, simple
  documentation drafting, bounded polling, and repetitive checks. MUST NOT use
  Luna as the blanket default for implementation, security review, or ambiguous
  reasoning.
- Cost guidance: Luna is an optimization for bounded low-risk work, not a
  quality-neutral replacement for Terra.
- A named custom specialist TOML is the model and reasoning-effort source of
  truth. MUST NOT pass model or reasoning-effort overrides.
- `codexy-sentinel` remains `gpt-5.6-sol` / `xhigh`. MUST NOT use Ultra.
  Custom-agent invocations MUST use `fork_turns="none"` or a positive bounded
  count with a self-contained handoff.

## Recipient Model Routing

- Configured UI model is authoritative; active child/parent thread ledger entries MUST
  record each destination owner's configured UI `model` and `thinking` separately
  from historical actual `turn_context` model and per-message overrides.
- Every `send_message_to_thread` call, parent-to-child or child-to-parent, MUST
  explicitly pass the recipient's configured UI `model` and `thinking`. MUST NOT
  infer either from historical actual `turn_context` state, the sender, or ambient defaults.
- Parent-to-generic-child delivery MUST pass `model: "gpt-5.6-terra"` and
  `thinking: "high"`; child-to-root delivery MUST pass `model: "gpt-5.6-sol"`
  and `thinking: "high"`.
- Captured #433 parent-to-generic-child evidence: configured_ui_model="gpt-5.6-terra"; actual_turn_context_model="gpt-5.6-sol"; per_message_model="gpt-5.6-terra"; send_message_to_thread({ threadId: "child-433", model: "gpt-5.6-terra", thinking: "high" }).
- Reverse child-to-root evidence: configured_ui_model="gpt-5.6-sol"; actual_turn_context_model="gpt-5.6-terra"; per_message_model="gpt-5.6-sol"; send_message_to_thread({ threadId: "root-433", model: "gpt-5.6-sol", thinking: "high" }).

## Read Next

MUST read these relative references before acting on the matching surface:

- `references/classification-and-control.md` for classification, goal, plan,
  child execution, multi-agent, codegraph, LSP, and sentinel discipline.
- `references/goal-transition-reporting.md` for delegated parent goal-report receipts.
- `references/thread-and-worktree-routing.md` for parent/child boundaries,
  thread discovery, Codex app worktree preflights, and worktree rules.
- `references/orchestration-loop.md` for intake, plan, dispatch, integration,
  verification, finish, failure modes, and handoffs.
- `references/runtime-heartbeats.md` for external waits.
- `references/parent-stop-preflight.md` for ownership checks before implementation edits.
- `references/execution-budget.md` for finite child execution and termination.

## Classification Gate

MUST run `$task-classification` before this skill starts setup, validation, release,
delegation, implementation, PR handling, review-response routing, or merge
coordination for Codexy work. Classification evidence MUST name the lane type,
owner decision, atomic scope, required skills, required tools or evidence,
first allowed action, and any stop blocker. Missing classification before
setup, validation, release, or other workflow actions is a workflow defect:
MUST stop, classify, and only then MUST continue through the matching Codexy workflow.

## Packaged Agents

Codexy ships specialist agent definitions as plugin-packaged Codex custom-agent
TOML files at `plugins/codexy/agents/<name>.toml`, with discovery metadata in
`plugins/codexy/agents/catalog.toml`; MUST keep one specialist agent per file.
`plugins/codexy/agents/openai.yaml` is the plugin invocation interface, not a
specialist worker.

Installed Codexy specialists require the stable registration bridge and an
independent schema/invocation preflight. MUST read
`references/agent-registration.md` before registering, updating, uninstalling,
diagnosing, or invoking a packaged specialist. MUST NOT treat
`plugins/codexy/.codex/agents` as installed custom agents.

## Required Control Plane

- MUST establish the goal before implementation. If `create_goal` is available,
  MUST use it directly for non-trivial delegated or orchestrated lanes; MUST use
  `get_goal` to inspect active goal state when needed; MUST use `update_goal` only
  when completion or true blockage is proved.
- MUST maintain a visible todo list with real `update_plan` or todo-tool state for
  any non-trivial task when available. Prose-only todo text is insufficient
  unless the todo/plan tool is unavailable and the fallback is reported.
- MUST treat asynchronous completion as event waits, not blockers. When an eligible external gate outlives the turn, parent orchestrators and child owners MUST follow `references/runtime-heartbeats.md`. Live Sentinel observation MUST be read-only and event-driven. Generic child and ledger polling remains permitted. Both the child owner and the root orchestrator MUST NOT message, interrupt, replace, follow up with, or poll a live Sentinel. A live Sentinel MUST report its own terminal `PASS`, `BLOCK`, or `UNOBSERVABLE` result naturally.
- In long multi-issue or multi-PR polling loops, MUST use
  `$token-efficient-orchestration` for preserving all proof gates while
  carrying only current deltas.
- Opening a PR is not completion when the requested outcome includes
  completion, merge, default Codexy merge flow, or no explicit stop/wait/
  draft-only/leave-open instruction.
- If a completion or handoff artifact reports completion while a matching clean
  PR remains open, validate it with
  `scripts/validate-plugin-config --check-completion-handoff --handoff-file <report> --pr-state-file <gh-pr-view-json>`.
  If the report discusses addressed review feedback, the PR state evidence
  MUST include GraphQL `reviewThreads.nodes`.

## Active Child Thread Ledger
Orchestration MUST maintain a durable active/waiting child thread ledger across normal polling, compaction recovery, dreaming rehydration, and parent handoffs.
Active child Codex app threads MUST be capped at 5. Orchestrators MUST count
only active/waiting Codex app child threads against that cap and MUST NOT create, continue, or resume a sixth active child thread until another active child thread has finished, stopped, or been explicitly removed from the ledger.
Packaged specialist subagents MUST NOT be counted as active
child Codex app threads.

Before creating a new child Codex app thread, orchestration MUST check the ledger and current issue/PR state for an existing issue/PR owner thread, MUST treat it as the existing owner thread, and MUST reuse it when present. If that owner is usable, orchestration MUST reuse or continue it instead of creating a duplicate owner.
Replacement child threads MUST be created only after existing owner evidence is inspected and the old owner is stopped, unusable, or explicitly superseded.
Each ledger entry MUST include issue/PR, thread id, status, owner state,
blocker, latest evidence, and next action. It MUST also include canonical
worktree CWD, frozen HEAD, clean/index state, every referencing specialist or
Sentinel task id, and explicit release/archive state. Normal polling MUST refresh
these fields from current thread, worktree, issue, PR, and review evidence.
Blocked/rate-limited child lanes MUST be continued through the existing owner when possible, with blocker and next action kept current in the ledger. Packaged specialist subagents
MUST NOT count against the child-thread cap, but every active or waiting
specialist or Sentinel that references a worktree MUST keep its reservation
active. Compaction recovery and dreaming rehydration MUST rebuild the ledger
before dispatching more child work or claiming no active child work remains.
Completed child threads MUST remain reserved until every referencing task is
terminal and explicitly archived or released. The orchestrator MUST record an
unavailable archive/delete surface as unresolved reservation evidence; it MUST
NOT silently recycle that worktree.

## Multi-Agent And Reviewer Gate

Delegation boundary: The root orchestrator MAY create child threads. A child implementation thread MAY spawn bounded first-level specialist helpers or Sentinel reviewers. Every helper or Sentinel assignment MUST include the hard instruction: `MUST NOT spawn, delegate to, or create any additional agent, helper, reviewer, task, or thread.`

MUST use multi-agent dispatch for bounded specialist help inside the current thread
when the lane does not need its own branch or PR and has separable research,
implementation, QA, verification, review, or review-feedback work. A `spawn_agent`
subagent is a helper, reviewer, explorer, or worker inside the current orchestration
context. Subagents are not child-owned implementation owners. A
subagent MUST NOT be treated as a
Codex subthread/worktree owner.

When a packaged Codexy specialist role is available and the task clearly falls
within that specialist's stated scope, the owning thread MUST use that
specialist or record a concrete skip rationale tied to scope, atomicity,
unavailable tooling, or lack of a matching task. A generic "not needed" note is
insufficient. Situational routing is:

- MUST use `codexy-cartographer` for repository, file, dependency, or ownership
  mapping before broad exploration.
- MUST use `codexy-pathfinder` for ambiguous, multi-step, cross-surface, or
  approach-selection work before implementation.
- MUST use `codexy-architect` for boundary, schema, MCP, LSP, plugin
  architecture, or long-lived extension-point changes.
- MUST use `codexy-tracer` for failing behavior, broken automation, root-cause
  investigation, or reproduction-heavy defects.
- MUST use `codexy-warden` for workflows, shell commands, credentials, remote
  MCP endpoints, untrusted input, repository permissions, install scripts, local
  state mutation, or generated evidence with security implications.
- MUST use `codexy-auditor` after implementation for acceptance-criteria,
  readiness, and observable verification passes across CLI, config, GitHub,
  browser, app, plugin, documentation, or workflow surfaces.
- MUST use `codexy-scribe` for docs, handoff, PR, release note, or
  user-facing workflow drafting after behavior is known.
- MUST use `codexy-forge` for scoped implementation edits after issue, branch,
  worktree, plan, and acceptance criteria are clear.
- MUST use `codexy-weaver` for reconciling parallel lanes, conflict checks,
  main updates, or merge sequencing.
- MUST use `codexy-sculptor` for refactor-heavy changes, large-file splits,
  helper extraction, or LOC-boundary cleanup.
- MUST use `codexy-shipwright` for release, packaging, version, marketplace,
  manifest, tag, or rollback work.
- MUST use `codexy-sentinel` as the final reviewer gate for every non-trivial
  atomic unit before handoff, PR readiness, completion, or parent acceptance.

If `spawn_agent` supports the Codexy role, invoke specialists by exact agent
type with no or bounded history, such as `spawn_agent(agent_type="codexy-sentinel", message="Review the current diff, exact head, scope, verification output, and evidence. MUST NOT spawn, delegate to, or create any additional agent, helper, reviewer, task, or thread.", fork_turns="none")`,
`spawn_agent(agent_type="codexy-pathfinder", message="Produce an atomic plan and verification checklist. MUST NOT spawn, delegate to, or create any additional agent, helper, reviewer, task, or thread.", fork_turns="3")`, or
`spawn_agent(agent_type="codexy-cartographer", message="Map the relevant files. MUST NOT spawn, delegate to, or create any additional agent, helper, reviewer, task, or thread.", fork_turns="none")`.

If `spawn_agent` or the requested Codexy `agent_type` is unavailable, MUST follow
`references/agent-registration.md`, MUST run the installed plugin's packaged
`scripts/bootstrap-codexy-agents`, MUST honor `RESTART_REQUIRED`, and MUST prove
the exact native role in a fresh task. MUST NOT substitute a generic agent for
a packaged Codexy specialist or Sentinel.

MUST end every non-trivial atomic unit with the packaged Codexy reviewer agent
defined in `plugins/codexy/agents/codexy-sentinel.toml`. The reviewer gate MUST
review the current diff, exact head or file state, lane scope, touched implementation-file
LOC evidence, verification outputs, and evidence before handoff, PR readiness,
completion, or parent acceptance. The parent may verify the evidence, but it
MUST NOT replace the owning lane's reviewer pass with parent-only readthrough,
an arbitrary reviewer, generic review role, or stale reviewer output.

Packaged Sentinel waits MUST end in one explicit lane status: `PASS`, `BLOCK`,
or `UNOBSERVABLE`. The owning lane MUST bound its wait, MUST report the
reviewer name and exact head, and MUST keep push/readiness blocked for `BLOCK` or
`UNOBSERVABLE` unless a maintainer explicitly approves a fallback. A delayed,
pending, stuck, or unobservable Sentinel MUST NOT be treated as approval.
The Sentinel MUST review only this issue's acceptance criteria, authorized behavior/files, current PR head or current diff, and necessary regressions.
Every BLOCK finding MUST map to an in-scope acceptance criterion.
Unrelated edge cases MUST be documented as non-blocking follow-up issues and MUST NOT block this lane.
Recurring same-class defects MUST receive one structural root-cause repair rather than phrase patches; MUST ask parent before widening files.

## Codegraph And LSP

For repository code exploration, MUST use the packaged Codexy `codegraph` MCP when
it is available before falling back to text search. MUST identify files, import
edges, and nearby implementation surfaces with codegraph output, then MUST confirm
with direct file reads before editing.

For language-aware code edits, MUST use Codexy `lsp` to check the matching server
registration and status when it is callable. If the matching server is not
callable, not installed, or not applicable, include `lsp_status` output or
explicit unavailable/not applicable evidence in the handoff or PR readiness
packet.

If a packaged MCP such as `lsp` or `codegraph` is expected or registered but
not callable in the active session, follow root `AGENTS.md` dogfooding policy:
MUST capture both surfaces as evidence and carry the exposure mismatch instead of
presenting a quiet fallback as normal.

## Parent Stop Preflight

MUST follow `references/parent-stop-preflight.md` before implementation edits.
MUST run `scripts/validate-plugin-config --check-child-lane-ownership --evidence-file <path>` when that reference requires ownership evidence.

## Completion Guard

## Event-driven token and quota containment

The root/orchestrator MUST NOT retain a persistent long-running goal, MUST NOT autonomously poll, and MUST process only compact deltas for terminal child state, Sentinel verdict, PR creation, new HEAD, GitHub check-state change, actionable review-feedback change, or review-thread resolution; ordinary progress and unchanged waiting MUST NOT wake the parent. Every delta MUST carry a stable event identity and exact task ids. Parent-message failure MUST emit exactly one terminal unavailable report and MUST NOT retry; no full conversation transfer or full agent-tree listing. A parent or child MUST NOT retain an active goal or plan during an external-gate wait. A child MUST use short-lived execution goals only. Once code, proof, push, and review-response work is complete and only an external gate remains, the child MUST send exactly one terminal parent handoff, call `update_goal(complete)`, and end its plan before waiting. A child external-gate wait MUST end its active goal and plan before waiting; when a runtime monitor is absent, it MUST return control. A runtime monitor lives outside goals. A registered heartbeat automation route uses its automation id, target thread, bounded schedule, and state fingerprint; a heartbeat automation route MUST NOT require a persistent exec/session id or same-process resume. A separate process-backed route requires those fields plus a next deadline. Both MUST suppress unchanged observations without assistant turns. A qualifying event starts a fresh short-lived execution goal. `blocked` is reserved for a repeated genuine execution impasse and MUST NOT represent an asynchronous external-gate wait.

When a Material child event arrives—terminal child state, actionable review feedback, or replacement-owner availability—the parent MUST validate the stable event identity and consume it in the same turn. To consume the event, the parent MUST perform the authorized parent-owned next action, such as route actionable review feedback, start a replacement owner, or resolve a verified gate, or MUST record a concrete execution blocker. An acknowledgement-only output MUST NOT satisfy consumption. Duplicate stable event identities MUST remain deduplicated with no parent action, and unchanged continuation observations MUST NOT create assistant turns.

Before creating a child, inspect archive candidates and the active reservation ledger; MAY archive only terminal, unreferenced, clean and unreserved worktree lanes with no open PR or pending gate, MUST NOT archive PR owners or dirty/reserved candidates, and MUST record the decision in setup evidence. A child implementation lane MUST use a short-lived child implementation goal. After Sentinel BLOCK, the usable existing owner MUST record the `block` and update the plan to a repair step, add faithful RED coverage, repair, rerun terminal proof, then invoke exactly one fresh Sentinel review for the new file state or head.
Event-driven refresh MUST update only from qualifying changes; a failed parent message MUST NOT retry the parent message, there MUST be no full agent-tree listing, and orchestration MUST inspect archive candidates and the active reservation ledger.
Runtime polling evidence and terminal handoff rules are defined in
`references/goal-transition-reporting.md`; MUST follow that contract.

MUST NOT mark a plan step complete until its evidence has been inspected.
MUST use `update_goal` only with an active or user-requested goal and current proof;
MUST reserve `blocked` for repeated true impasses requiring user input or external
state change.
