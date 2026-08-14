---
name: orchestration
description: MUST use first for Codexy task classification and then for goals, plans, issue-sized lanes, threads, worktrees, compaction ledgers, token discipline, and orchestrator-led execution loops.
---

# Orchestration

## Purpose

MUST run the current plugin-invoking Codex thread as the root/orchestrator for
goal-oriented work. MUST NOT spawn or assign a separate orchestrator agent. The
invoking Codex thread owns intent, decomposition, routing, evidence integration,
and final completion claims. Specialists and separate Codex thread/worktree lanes
own bounded atomic units only; Root `AGENTS.md` owns repo-wide dogfooding policy.

## GPT-5.6 Routing Matrix

The closed machine-owned routing authority is
`references/child-routing-policy.json`; its paired current measurement evidence
is `references/routing-evaluation-results.json`. This matrix is the human-readable
workflow projection and MUST NOT be parsed as policy.

Bounded review selection is separately owned by the closed
`references/review-profiles.json` and `references/workflow-review-classification.json`
contracts. The selected profile derives from the exhaustive typed classification
record, not generic child routing or this prose projection.

- Root/orchestrator: MUST use `gpt-5.6-sol` for decomposition, risk decisions,
  integration, and completion.
- Generic implementation children MUST request `gpt-5.6-terra` with `reasoning_effort: "high"` as the fail-closed default. Promotion above Terra/high is allowed only as an explicit exception selected by complete validated measurement.
- A matching named specialist MUST be selected before generic child routing; its TOML remains authoritative.
- Candidate simple work MUST use `gpt-5.6-luna` with `reasoning_effort: "max"` only when fixed scope, deterministic oracle, low-risk/reversible boundary, and no unresolved domain, security, permission, release, or ownership decision all hold.
- Candidate general work MUST compare Terra/high, Terra/xhigh, and Terra/max and select the lowest effort meeting measured quality and economics gates.
- Measurement gate: promotion above Terra/high MUST have zero P0/P1 defects, at least 95% acceptance, either a five-point first-pass gain or 20% fewer repairs, and no more than 1.5x median cost or wall time.
- Ambiguous, high-risk, or incomplete classification MUST fail closed to root or a named specialist; it MUST NOT select Luna.
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
- Before every Codex app `create_thread` or `send_message_to_thread` call, the
  parent MUST load the policy and send a closed typed classification request with
  `codex_thread_operation` set to that exact operation and
  `codex_thread_capabilities` set to the app surface's advertised
  model/thinking pairs. It MUST fail closed without a child-thread call when its
  result lacks a generic recipient binding; an unavailable Luna/max candidate
  falls back to advertised Terra/high or the safe route.
- The generic resolver result binds the same `codex_thread_operation`, `model`,
  and `thinking` that the parent MUST pass explicitly to that Codex app call.
  A named-specialist or safe-route result has no generic thread override.
- Parent-to-generic-child delivery uses `gpt-5.6-terra`/`high`; child-to-root
  delivery uses `gpt-5.6-sol`/`medium`.
- Captured #433 parent-to-generic-child evidence: configured_ui_model="gpt-5.6-terra"; actual_turn_context_model="gpt-5.6-sol"; per_message_model="gpt-5.6-terra"; send_message_to_thread({ threadId: "child-433", model: "gpt-5.6-terra", thinking: "high" }).
- Reverse child-to-root evidence: configured_ui_model="gpt-5.6-sol"; actual_turn_context_model="gpt-5.6-terra"; per_message_model="gpt-5.6-sol"; send_message_to_thread({ threadId: "root-433", model: "gpt-5.6-sol", thinking: "medium" }).

## Read Next

MUST read these relative references before acting on the matching surface:

- `references/task-classification.md` and `references/classification-and-control.md` for classification, goal, plan, child execution, multi-agent, codegraph, LSP, and Sentinel discipline.
- `references/goal-transition-reporting.md` for delegated parent goal-report receipts.
- `references/plugin-public-contracts.md` when an installed extension invokes `$orchestration` across the Codex plugin boundary.
- `references/thread-and-worktree-routing.md` for parent/child boundaries, thread discovery, Codex app worktree preflights, and worktree rules.
- `references/orchestration-loop.md` for intake, plan, dispatch, integration, verification, finish, failure modes, and handoffs.
- `references/runtime-heartbeats.md` for external waits.
- `references/parent-stop-preflight.md` for ownership checks before implementation edits.
- `references/execution-budget.md` for finite child execution and termination.
- `references/token-efficient.md` for compact event deltas and token discipline.
- `references/plain-language-user-replies.md` and `references/natural-korean-responses.md` for user-facing replies and separate machine-readable evidence.
- `references/child-routing-policy.json`, `references/routing-evaluation-corpus.json`, `references/routing-evaluation-results.schema.json`, and `references/routing-evaluation-results.json` for structured child-routing selection and frozen paired measurement.
- `references/review-profiles.json` and `references/workflow-review-classification.json` for structured review budgets and exhaustive typed profile selection.

## Classification Gate

MUST classify the lane through this skill before setup, validation, release,
delegation, implementation, PR handling, review-response routing, or merge
coordination for Codexy work. Classification evidence MUST name the lane type,
owner decision, atomic scope, required skills, required tools or evidence,
first allowed action, and any stop blocker. Missing classification before
setup, validation, release, or other workflow actions is a workflow defect:
MUST stop, classify, and only then MUST continue through the matching Codexy workflow.

## Authority Boundary

`references/task-classification.md` is the authoritative ownership contract; its formal classification gate MUST run before setup or action.

## Packaged Agents

MUST read `references/agent-registration.md` before registering, updating,
uninstalling, diagnosing, or invoking a packaged specialist.

## Required Control Plane

- MUST establish the goal before implementation. If `create_goal` is available,
  MUST use it directly for non-trivial delegated or orchestrated lanes; MUST use
  `get_goal` to inspect active goal state when needed; MUST use `update_goal` only
  when completion or true blockage is proved.
- MUST maintain a visible todo list with real `update_plan` or todo-tool state for
  any non-trivial task when available. Prose-only todo text is insufficient
  unless the todo/plan tool is unavailable and the fallback is reported.
- MUST follow [event-driven-waits.md](references/event-driven-waits.md) for
  asynchronous waits, host-transition recovery, and live-Sentinel observation.
- In long multi-issue or multi-PR polling loops, MUST preserve all proof gates while carrying only current deltas.
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

Before creating a new child Codex app thread, orchestration MUST follow
[child owner reuse](references/child-owner-reuse.md).
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
- MUST use `codexy-architect` for boundary, schema, MCP, LSP, plugin
  architecture, or long-lived extension-point changes.
- MUST use `codexy-warden` for workflows, shell commands, credentials, remote
  MCP endpoints, untrusted input, repository permissions, install scripts, local
  state mutation, or generated evidence with security implications.
- MUST use `codexy-auditor` after implementation for acceptance-criteria,
  readiness, and observable verification passes across CLI, config, GitHub,
  browser, app, plugin, documentation, or workflow surfaces.
- When optional `codexy-github` is installed, MUST use its `codexy-weaver` for GitHub integration, conflict checks, main updates, or merge sequencing.
  Core-only installations MUST report GitHub integration unavailable instead of depending on an extension-private role.
- MUST use `codexy-shipwright` for release, packaging, version, marketplace,
  manifest, tag, or rollback work.
- MUST select exactly the reviewer prescribed by machine-owned
  `references/review-profiles.json`: light selects no LLM reviewer, standard
  selects `codexy-inspector`, and strict selects `codexy-sentinel`.

Orchestration owns planning and approach selection. A generic owning child uses
the engineering workflow for diagnosis, TDD, QA, and refactoring, and directly
owns its scoped implementation, documentation, and handoff; it MUST NOT recreate
a removed specialist as an alias.

If `spawn_agent` supports the Codexy role, invoke specialists by exact agent
type with no or bounded history, such as `spawn_agent(agent_type="codexy-sentinel", message="Review the current diff, exact head, scope, verification output, and evidence. MUST NOT spawn, delegate to, or create any additional agent, helper, reviewer, task, or thread.", fork_turns="none")`,
`spawn_agent(agent_type="codexy-cartographer", message="Map the relevant files. MUST NOT spawn, delegate to, or create any additional agent, helper, reviewer, task, or thread.", fork_turns="none")`.

If `spawn_agent` or the requested Codexy `agent_type` is unavailable, MUST follow
`references/agent-registration.md`, MUST run the installed plugin's packaged
`scripts/bootstrap-codexy-agents`, MUST honor `RESTART_REQUIRED`, and MUST prove
the exact native role in a fresh task. MUST NOT substitute a generic agent for
a packaged Codexy specialist or Sentinel.

MUST end every non-trivial atomic unit with machine-owned
`references/review-profiles.json`: light has no LLM reviewer; standard and
strict each have one selected-reviewer full review and at most one delta recheck. The selected
reviewer gate MUST review the current diff, exact head or file state, lane
scope, touched implementation-file LOC evidence, verification outputs, and
evidence before handoff, PR readiness, completion, or parent acceptance. The
parent MUST NOT add a second reviewer or replace the selected reviewer with
parent-only readthrough, an arbitrary reviewer, generic review role, or stale
reviewer output.

Selected profile reviewer terminal results MUST be `PASS`, `BLOCK`, or `UNOBSERVABLE`.
Non-terminal `PENDING`/`RUNNING` observation and same-reviewer retention MUST
follow `references/classification-and-control.md`. The owning lane MUST keep
push/readiness blocked until `PASS` or an explicitly approved terminal fallback.
The selected reviewer MUST review only this issue's acceptance criteria, authorized behavior/files, current PR head or current diff, and necessary regressions.
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

## Event-driven containment

MUST follow [event-driven-containment.md](references/event-driven-containment.md)
for child-event handling, nonterminal waits, archive decisions, and post-BLOCK
repair/delta-review control.
