---
name: codex-orchestration
description: MUST use when coordinating Codex plugin calls, long-running goals, issue-sized decomposition, multi-agent or multi-thread execution, worktrees, todo/update_plan tracking, and orchestrator-led implementation loops.
---

# Codex Orchestration

## Purpose

MUST run the current plugin-invoking Codex thread as the orchestrator for
goal-oriented work. MUST NOT spawn or assign a separate orchestrator agent. The
invoking Codex thread owns intent, decomposition, routing, evidence
integration, and final completion claims. Specialist subagents and separate
Codex thread/worktree lanes own bounded atomic units only.

Root `AGENTS.md` owns repo-wide dogfooding policy. This skill supplies the
execution loop and MUST be read with root `AGENTS.md`.

## Read Next

MUST read these relative references before acting on the matching surface:

- `references/classification-and-control.md` for classification, goal, plan,
  child execution, multi-agent, codegraph, LSP, and sentinel discipline.
- `references/goal-transition-reporting.md` for delegated parent goal-report receipts.
- `references/thread-and-worktree-routing.md` for parent/child boundaries,
  thread discovery, Codex app worktree preflights, and worktree rules.
- `references/orchestration-loop.md` for the intake, plan, dispatch,
  integrate, verify, and finish loop plus handoff templates.

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
- MUST treat Codex connector review, child-thread work, queued worktree/thread
  setup, and asynchronous tool completion as active waiting states, not blockers.
- Status observation of a running packaged Sentinel MUST be read-only. It MUST
  NOT send messages, interrupts, follow-up prompts, or other mutations.
- A live Sentinel MUST remain active until it produces its own `PASS`, `BLOCK`,
  or `UNOBSERVABLE` terminal result; delayed output alone MUST NOT cause `UNOBSERVABLE`.
- Parent policy MUST use event-driven terminal deltas and MUST NOT poll a running Sentinel.
- In long multi-issue or multi-PR event-driven loops, MUST use
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
Orchestration MUST maintain a durable active/waiting child thread ledger across event-driven observation, compaction recovery, dreaming rehydration, and parent handoffs.
Active child Codex app threads MUST be capped at 5. Orchestrators MUST count
only active/waiting Codex app child threads against that cap and MUST NOT create, continue, or resume a sixth active child thread until another active child thread has finished, stopped, or been explicitly removed from the ledger.
Packaged specialist subagents MUST NOT be counted as active
child Codex app threads.

Before creating a new child Codex app thread, orchestration MUST check the ledger and current issue/PR state for an existing issue/PR owner thread, MUST treat it as the existing owner thread, and MUST reuse it when present. If that owner is usable, orchestration MUST reuse or continue it instead of creating a duplicate owner.
Replacement child threads MUST be created only after existing owner evidence is inspected and the old owner is stopped, unusable, or explicitly superseded.
Each ledger entry MUST include issue/PR, thread id, status, owner state,
blocker, latest evidence, and next action. It MUST also include canonical
worktree CWD, frozen HEAD, clean/index state, every referencing specialist or
Sentinel task id, and explicit release/archive state. Material events MUST refresh
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

MUST use multi-agent dispatch for bounded specialist help inside the current thread
when the lane does not need its own branch or PR and has separable research,
implementation, QA, verification, review, or review-feedback work. A
`spawn_agent` subagent is a helper, reviewer, explorer, or worker inside the
current orchestration context. A `spawn_agent` subagent MUST NOT be treated as a
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
type with no or bounded history, such as `spawn_agent(agent_type="codexy-sentinel", message="Review the current diff, exact head, scope, verification output, and evidence.", fork_turns="none")`,
`spawn_agent(agent_type="codexy-pathfinder", message="Produce an atomic plan and verification checklist.", fork_turns="3")`, or
`spawn_agent(agent_type="codexy-cartographer", message="Map the relevant files.", fork_turns="none")`.

If `spawn_agent` or the requested Codexy `agent_type` is unavailable, MUST report
that the Codexy agents have not been registered in the active Codex config and
fall back to packaged TOML/catalog context without claiming native-agent
success.

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

MUST run this checkpoint before any implementation edit when a lane may need a
branch, worktree, PR, durable child context, or review-response ownership:

1. MUST name the atomic lane and decide ownership as `parent-owned` or
   `child-owned`.
2. If the lane is `child-owned`, the parent may prepare issue text, branch
   names, worktree requests, handoff text, and acceptance criteria, but it
   MUST NOT patch implementation files, create implementation branches or
   worktrees in the parent context, or read implementation surfaces as setup
   for a parent patch.
3. If parent draft implementation diff or setup artifacts already exist for a
   child-owned lane, MUST preserve the evidence, disclose the workflow defect,
   MUST inspect overlap with user or other-agent work, and MUST route the draft state
   to the child instead of continuing implementation.
4. When handoff or final-answer evidence for a child-owned PR includes
   parent-authored implementation, implementation setup, or review-response
   commits, MUST run
   `scripts/validate-plugin-config --check-child-lane-ownership --evidence-file <path>`.
5. A failed first search for thread or worktree tooling is not proof that the
   tooling is unavailable. MUST continue discovery before reporting a blocker.

## Child Thread Titles

- After a forked Codex worktree child thread finishes setup and a thread id is
  available, the orchestrator MUST rename it with `set_thread_title` when the
  tool is available.
- The child thread title MUST clearly include the project, issue number, and
  lane purpose so users can distinguish concurrent child threads, such as
  `Codexy #52 refactoring skill agent lane`.
- If thread title renaming is unavailable, mention that limitation in the
  orchestration status or child handoff and continue with the lane.
- Child thread title renaming is a clarity policy, not a merge blocker for
  otherwise complete implementation work.

## Completion Guard

MUST NOT mark a plan step complete until its evidence has been inspected by the
orchestrator. MUST use `update_goal` only when that tool is available, an active or
user-requested goal exists, and every explicit requirement has current matching
proof. Reserve `blocked` for repeated true impasses where meaningful progress
requires user input or an external state change.

## Failure Modes

- Starting setup, delegation, implementation, validation, PR handling, review
  response, or merge coordination before `$task-classification`.
- Subagents are not child-owned implementation owners.
- Treating subagents as child-owned Codex thread/worktree owners.
- Marking a goal blocked because review, child work, worktree/thread setup, or
  another asynchronous tool is pending.
- Treating expected or registered MCPs as ordinary unavailable tools when the
  callable Codex surface does not expose them.
- Starting parent implementation patches for a lane that needs its own child
  thread, worktree, branch, or PR, then delegating only after files changed.
- Treating parent-only readthrough, arbitrary reviewer agents, generic review
  roles, or stale reviewer output as the packaged Codexy reviewer gate.
- Reporting completion while review comments, open threads, stale PR heads, or
  unverified claims remain unresolved.
