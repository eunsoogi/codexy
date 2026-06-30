---
name: codex-orchestration
description: Use when coordinating Codex plugin calls, long-running goals, issue-sized decomposition, multi-agent or multi-thread execution, worktrees, todo/update_plan tracking, and orchestrator-led implementation loops.
---

# Codex Orchestration

## Purpose

Run the current plugin-invoking Codex thread as the orchestrator for
goal-oriented work. Do not spawn or assign a separate orchestrator agent. The
invoking Codex thread owns intent, decomposition, routing, evidence
integration, and the final completion claim. Specialist subagents and separate
Codex thread/worktree lanes own bounded atomic units only.

Root `AGENTS.md` owns repo-wide dogfooding policy, including governing
instruction failures, expected-but-uncallable tool surfaces, Codex app
thread/worktree preflights, merge-through-completion expectations, and
parent/child ownership boundaries. This skill provides execution mechanics for
the orchestration loop and must be read together with root `AGENTS.md`.

## Classification Gate

Run `$task-classification` before this skill starts setup, validation, release,
delegation, implementation, PR handling, review-response routing, or merge
coordination for Codexy work. Classification evidence must name the lane type,
owner decision, atomic scope, required skills, required tools or evidence,
first allowed action, and any stop blocker. Missing classification before
setup, validation, release, or other workflow actions is a workflow defect:
stop, classify, and only then continue through the matching Codexy workflow.

Codexy ships specialist agent definitions as plugin-packaged Codex custom-agent
TOML files at `plugins/codexy/agents/<name>.toml`, with discovery metadata in
`plugins/codexy/agents/catalog.toml`. Keep one specialist agent per file.
`plugins/codexy/agents/openai.yaml` is the plugin invocation interface, not a
specialist worker. Installed Codexy agents become native Codex `spawn_agent`
roles only after the user's Codex config registers those packaged TOMLs through
`[agents.<codexy-name>] config_file = "<installed-plugin>/agents/<codexy-name>.toml"`.
Use `skills/codex-orchestration/scripts/register-codexy-agents` from the
installed plugin to add or remove Codexy's managed config block safely. Do not
treat `plugins/codexy/.codex/agents` as installed custom agents: Codex
discovers native project custom agents from active project `.codex/agents`, and
plugin-provided Codexy agents use the config-file registration path above.
Packaged Codexy agent filenames and `name` fields are already distinctive
Codexy agent types, such as `codexy-sentinel`, `codexy-pathfinder`, and
`codexy-cartographer`.

To register Codexy agents from an installed plugin, run:

```sh
skills/codex-orchestration/scripts/register-codexy-agents
```

The script writes a managed block to `${CODEX_HOME:-$HOME/.codex}/config.toml`,
backs up an existing config before changing it, refuses unmanaged
`[agents.<codexy-name>]` conflicts, registers every catalog agent under its
packaged Codexy name, supports `--dry-run`, and supports `--uninstall`.
Restart Codex or start a fresh session after registration before expecting new
`spawn_agent` agent types to appear.

## Parent And Child Thread Boundary

- The plugin-invoking Codex thread is the orchestrator. It creates or confirms
  issues, assigns branches, delegates lanes, opens PRs when appropriate,
  requests Codex review, performs parent verification, coordinates squash merge,
  and syncs `main`.
- A child Codex worktree thread owns implementation edits, local verification,
  and review-response fixes for its assigned issue or lane.
- Independent requested outcomes MUST be decomposed into separate issue-sized
  atomic child lanes before child thread, worktree, branch, or PR creation.
  Each atomic lane gets its own issue or explicit issue-sized scope, branch,
  worktree or thread when needed, and PR. Bundling independent outcomes into
  one child lane is a workflow violation unless a maintainer explicitly scopes
  those outcomes as one atomic lane before implementation begins.
- For any lane that needs its own branch, worktree, PR, or long-running
  implementation context, the orchestrator MUST create, fork, or assign the
  owning child thread before implementation patches begin. The orchestrator
  MUST NOT make draft implementation edits first and delegate afterward.
- If Codex connector or human review feedback flags a child-owned PR, the
  orchestrator MUST route the feedback back to the owning child thread with the
  PR number, latest head SHA, relevant comments or review threads, expected
  return evidence, and stop condition.
- If the owning child thread becomes unresponsive, stale, or unable to return
  required review-response evidence, the orchestrator MUST stop and report the
  blocker, current PR head, child owner, last contact, and required next
  evidence. It MUST NOT patch the child-owned branch during recovery unless a
  maintainer explicitly reassigns implementation ownership to the orchestrator.
- The orchestrator MUST NOT directly fix child-owned review feedback unless the
  lane is explicitly reassigned to the orchestrator by a maintainer, or the
  feedback belongs to the orchestrator's own scoped lane.
- The orchestrator may resolve review threads only after child evidence proves
  the fix on the current head, or after a maintainer accepts a no-change
  rationale.
- If the orchestrator accidentally creates draft edits for a lane that should
  belong to a child thread, it MUST stop editing immediately, disclose the
  mistake in the parent thread, inspect whether the draft touches user or other
  agent work, and preserve the diff for handoff unless a maintainer explicitly
  asks to discard it. It must then hand the draft diff, allowed files, risks,
  and recovery state to the owning child thread instead of continuing the
  implementation.
- If a bundled child lane is discovered after dispatch or after edits begin,
  stop that lane immediately instead of continuing the umbrella lane. Preserve
  and report its draft state, including branch, worktree, diff summary,
  verification already run, unresolved risks, and any user or other-agent work
  overlap. Split the independent outcomes into atomic issues, threads,
  worktrees, branches, and PRs before implementation resumes. Maintainer
  re-scoping after dispatch or edits begin cannot bypass this recovery path; if
  a maintainer wants the outcomes handled together, start a new explicitly
  scoped atomic lane after the bundled lane has stopped and reported its draft
  state.
- Worktree lanes must stay issue-sized and atomic. Do not bundle review
  response work from one lane into another lane.

## Compaction And Continuation Guard

Treat loss of the active `@Codexy` or Codexy plugin workflow contract after
context compaction, goal continuation, or resume as a dogfooding defect. A
compacted continuation summary must preserve the active `$codex-orchestration`
contract, duplicate or no-active-work issue and PR state, parent/child
ownership boundaries, and the authoritative stop condition before any edit or
review request continues.

Before editing after compaction or continuation, re-check current GitHub state
for the issue and PR, especially docs or README lanes that may already be
merged, closed, or duplicate work. Also capture a fresh git preflight with
`pwd`, `git status --short --branch`, `git rev-parse HEAD`, `git rev-parse
origin/main`, and a small `git log --graph --oneline --decorate --all`
window. If the summary omits those facts, stop and rebuild the evidence instead
of treating the omission as a harmless fallback.

## Parent Stop Preflight

Run this checkpoint before any implementation edit when a lane may need a
branch, worktree, PR, durable child context, or review-response ownership:

Issue creation, PR or issue commenting, branch-name planning, handoff drafting,
and thread/worktree tool discovery are parent coordination. Creating an
implementation branch, creating an implementation worktree, reading
implementation files as setup for a parent patch, or editing files is
implementation setup and is not allowed for a child-owned or routing-only lane
unless a maintainer explicitly reassigns implementation ownership to the
parent.

1. Name the atomic lane and decide ownership as `parent-owned` or
   `child-owned`.
2. If the lane is `child-owned`, stop parent implementation before editing.
   The parent may prepare issue text, branch names, worktree requests, handoff
   text, and acceptance criteria, but it MUST NOT patch implementation files,
   create implementation branches or worktrees in the parent context, or read
   implementation surfaces as setup for a parent patch.
3. If any parent draft implementation diff already exists for a `child-owned`
   lane, preserve the diff as evidence, disclose the mistake, inspect for user
   or other-agent overlap, and route the draft diff to the child. Do not
   continue by "finishing the small fix" in the parent.
4. If parent implementation setup artifacts already exist for a child-owned
   lane, such as a draft worktree, parent-created implementation branch, or
   implementation-surface reads, disclose them as a workflow defect, preserve
   or clean up the artifacts as appropriate, inspect for user or other-agent
   overlap, and delegate to a clean child thread before implementation resumes.
5. Include the owner decision and stop condition in the handoff. PR readiness
   requires evidence that the child owner existed before implementation setup
   or patches began, or explicit recovery evidence for accidental parent setup
   or draft edits.
6. When handoff or final-answer evidence for a child-owned PR includes
   parent-authored implementation, implementation setup, or review-response
   commits, run
   `scripts/validate-plugin-config --check-child-lane-ownership --evidence-file <path>`
   against that evidence. Treat a failure as a workflow defect unless the same
   evidence records explicit maintainer reassignment to the parent.
7. A failed first search for thread or worktree tooling is not proof that the
   tooling is unavailable. Keep discovering the correct surface, inspect
   `tool_search` or registered tools again with narrower terms, or ask the
   maintainer for the exact thread/worktree surface. Do not create an
   implementation branch, read implementation files for a parent patch, or edit
   files while the stop condition is tool discovery or child-lane routing.

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

## Child Execution Discipline

Child implementation threads assigned a non-trivial lane MUST run their own
execution loop instead of treating the parent handoff as permission for ad hoc
edits.

- Create or maintain a lane-specific goal with the real Codex goal tools when
  they are available. Use `create_goal` to start the lane goal, `get_goal` to
  inspect active goal state when needed, and `update_goal` only when completion
  or true blockage is proved. A prose-only `Goal:` line or goal-looking status
  text is not evidence that goal tooling was used. If goal tooling is
  unavailable, write a visible textual goal with success criteria, keep it
  current in status updates, and report the unavailable-tool fallback in
  handoff evidence.
- Maintain real todo or plan state for multi-step delegated work when the
  tool is available, such as Codex `update_plan` or the active todo tool for
  the surface. Update statuses as work moves from discovery, to edit, to
  verification, to handoff. Prose-only `Todo:` text is not evidence that
  todo/plan tooling was used.
- Use multi-agent execution when the lane has independent research questions,
  disjoint implementation slices, QA or verification that can run in parallel,
  review gates, review-feedback validation, or any non-trivial atomic scope
  with separable subtasks. If multi-agent tooling is available, "not useful"
  is acceptable only with a concrete rationale tied to atomicity, tiny scope,
  or the absence of separable work; a generic manual fallback is not enough.
- Atomic trivial child tasks may stay lightweight, but substantial delegated
  work MUST NOT proceed as untracked edits without both real goal state and
  real todo/plan state when those tools are available. Using only one of goal
  or todo/plan is insufficient for a non-trivial child lane unless the missing
  tool is unavailable and the child reports that unavailability with its
  fallback.
- If a required execution tool is unavailable in the child thread, say so in
  the thread and use the closest available fallback. Do not silently skip the
  discipline.
- Before a child thread reports a non-trivial atomic lane as ready for parent
  handoff, PR readiness, completion, or parent acceptance, it MUST run the
  packaged Codexy reviewer agent defined by
  `plugins/codexy/agents/codexy-sentinel.toml` against the current lane diff, exact
  head or file state, lane scope, touched implementation-file LOC evidence,
  verification outputs, and available evidence.
  Do not substitute an arbitrary reviewer, generic review role, external
  review agent, parent-only readthrough, or stale reviewer output for this
  gate.
- The parent/orchestrator monitors evidence and merge gates. The child owns
  the implementation loop, local verification, and review-response fixes for
  its lane until the stop condition is met.

## Required Control Plane

- Establish the goal before implementation. If `create_goal` is available,
  use it directly for non-trivial delegated or orchestrated lanes; use
  `get_goal` to inspect active goal state when needed, and `update_goal` only
  when completion or true blockage is proved. Prose-only `Goal:` text is
  fallback documentation, not proof of goal-tool use. If goal tools are not
  available, keep a visible `Goal` note in the thread with success criteria,
  update it textually as evidence changes, and report that fallback.
- Treat waiting for Codex connector review, child-thread work, queued
  worktree/thread setup, or asynchronous tool completion as a non-blocking goal
  state. Keep the goal active, keep polling, send follow-up prompts when
  progress stalls, and continue the merge loop when evidence arrives.
- In long multi-issue or multi-PR polling loops, use
  `$token-efficient-orchestration` to carry deltas, exact ids, stale-context
  demotion, and one next action per lane while preserving all proof gates.
- For PR merge coordination, keep an active goal and real plan or todo item
  open through the review gate, merge gate, GitHub squash merge, branch
  deletion, and main sync. When the latest-head review gate is satisfied and
  merge prerequisites pass, advance the plan from review waiting into the
  merge/post-merge-sync step and continue unless the maintainer explicitly
  requested stop, wait, push only, no merge, draft only, or leaving the PR open.
- Opening a PR is not completion when the requested outcome includes
  completion, merge, default Codexy merge flow, or no explicit stop/wait/
  draft-only/leave-open instruction. If a handoff, final answer, or evidence
  artifact reports completion while a matching clean PR remains open, validate
  it with
  `scripts/validate-plugin-config --check-completion-handoff --handoff-file <report> --pr-state-file <gh-pr-view-json>`
  and fix the claim or continue through merge instead of marking the lane done.
  If the report discusses addressed review feedback, the PR state evidence must
  include GraphQL `reviewThreads.nodes`; addressed unresolved threads,
  including outdated-but-fixed threads, remain invalid unless the report
  documents an accepted no-change rationale.
- Reserve `blocked` for repeated true impasses where the orchestrator cannot
  make meaningful progress without user input or an external state change.
- Maintain a visible todo list with real `update_plan` or todo-tool state for
  any non-trivial task when available. Prose-only todo text is insufficient
  unless the todo/plan tool is unavailable and the fallback is reported.
- Decompose broad work into issue-sized atomic units before editing. If a
  request contains multiple independent outcomes, split them into separate
  atomic issues and child lanes before any child thread, worktree, branch, or
  PR is created, unless a maintainer explicitly says the outcomes are one
  atomic lane.
- Decide lane ownership before editing. If an atomic unit needs a branch,
  worktree, PR, or durable child context, dispatch the child thread first; do
  not use the parent thread for a preliminary implementation pass.
- Use multi-agent dispatch for bounded specialist help inside the current
  thread when the lane does not need its own branch or PR. A `spawn_agent`
  subagent is a helper, reviewer, explorer, or worker inside the current
  orchestration context; it is not a Codex subthread/worktree owner for an
  issue-sized child implementation lane that needs its own branch, durable
  worktree, PR, or review-response ownership. When true Codex
  thread/worktree ownership is requested or available, route those lanes
  through Codex thread/worktree tools instead of recording a subagent as the
  child owner. Multi-agent use is required when independent research
  questions, disjoint implementation slices, parallel QA or verification,
  review gates, review-feedback validation, or separable subtasks in a
  non-trivial atomic lane can be isolated. Use the packaged specialist agent
  files and lightweight catalog metadata as routing context. If `spawn_agent`
  supports the Codexy role, invoke specialists by exact agent type, such as
  `spawn_agent(agent_type="codexy-sentinel", message="Review
  the current diff, exact head, scope, verification output, and evidence.")` or
  `spawn_agent(agent_type="codexy-pathfinder", message="Produce an atomic plan and
  verification checklist.")`. Use
  `spawn_agent(agent_type="codexy-cartographer", message="Map the relevant files.")`
  for Codexy exploration.
  If `spawn_agent` or the requested Codexy
  `agent_type` is unavailable, report that the Codexy agents have not been
  registered in the active Codex config and fall back to packaged TOML/catalog
  context without claiming native-agent success. If true Codex thread/worktree
  tools are unavailable for a lane that requires them, record an exposure
  blocker and stop routing; do not substitute a subagent as the implementation
  owner.
- For repository code exploration, route threads and agents through the
  packaged Codexy `codegraph` MCP when it is available before falling back to
  ad hoc text search. Use codegraph output to identify files, import edges,
  and nearby implementation surfaces, then confirm with direct file reads
  before editing.
- For language-aware code edits, use Codexy `lsp` to check the matching server
  registration and status when it is callable. If the matching server is not
  callable, not installed, or not applicable, include the `lsp_status` output
  or explicit unavailable/not applicable evidence in the handoff or PR
  readiness packet.
- If a packaged MCP such as `lsp` or `codegraph` is expected or registered but
  not callable in the active session, follow root `AGENTS.md` dogfooding policy:
  capture both surfaces as evidence and carry the exposure mismatch instead of
  presenting a quiet fallback as normal.
- End every non-trivial atomic unit with the packaged Codexy reviewer agent
  from `plugins/codexy/agents/codexy-sentinel.toml`. The reviewer gate belongs inside
  the owning thread or child thread for that atomic unit and must review the
  current diff, exact head or file state, lane scope, touched
  implementation-file LOC evidence, verification outputs, and evidence before
  handoff, PR readiness, completion, or parent acceptance.
  The parent may verify the evidence, but it must not replace the owning
  lane's reviewer pass with a different ad hoc agent, arbitrary reviewer role,
  parent-only readthrough, or stale reviewer output.
- Use separate Codex thread/worktree decomposition when lanes can proceed
  independently, touch separate ownership areas, or need separate PRs. If
  worktree isolation is required and Codex thread tools are available, create
  or fork a Codex worktree thread instead of using only manual `git worktree`
  commands.
- Keep the invoking Codex thread as the orchestrator. It integrates child
  results, resolves conflicts, verifies final behavior, and decides whether
  work is complete.

## Orchestration Loop

1. Intake:
   - Run `$task-classification` and record classification before setup,
     delegation, implementation edits, PR handling, review-response routing, or
     merge coordination begins.
   - Read the latest user request, repository instructions, active issue, and
     relevant local skills.
   - Separate hard requirements, preferences, assumptions, and non-goals.
   - Identify the observable surface that proves the request worked.
   - For codebase discovery, use the Codexy `codegraph` MCP to map relevant
     files and neighbors when available.
2. Plan:
   - Create a short `update_plan` with atomic outcomes.
   - Mark exactly one step `in_progress`.
   - Carry classification evidence into the plan before branch, worktree,
     child-thread, implementation, PR, or review-response actions.
   - Split independent outcomes into separate issues and, when implementation
     can proceed independently, separate Codex thread/worktree lanes. Treat
     bundling independent requested outcomes as a workflow violation unless a
     maintainer explicitly scopes them as one atomic lane.
   - Mark each lane as parent-owned or child-owned before any implementation
     patch is made.
3. Dispatch:
   - Do not dispatch until classification evidence proves the lane type,
     owner, atomicity, required skills, required tools, and first allowed
     action.
   - Start specialist subagents only for bounded lanes that do not need their
     own branch or PR.
   - For issue-sized implementation lanes, start or fork a separate Codex
     thread in a worktree when the tool is available. Run the Thread Tool
     Discovery Procedure below before reporting unavailable tooling or choosing
     any fallback path. Manual `git worktree` setup, `codex` CLI commands, and
     ad hoc branch edits do not satisfy a required clean Codex thread unless a
     maintainer explicitly re-scopes the lane away from that requirement.
   - Before calling Codex app worktree or thread tools, run the Codex App
     Worktree Creation Preflight below. Do not retry failed setup by creating
     a second active owner until the pending or failed owner is resolved.
   - Do not dispatch a child lane that contains independent outcomes needing
     separate issues, branches, worktrees, threads, or PRs. Split the lane
     first, or stop and ask for maintainer scoping if atomic ownership is
     ambiguous.
   - Complete the lane assignment before implementation edits begin. A parent
     may prepare the issue, branch name, worktree path, and handoff text, but
     must not patch implementation files for the child-owned lane.
   - Give each lane an assignment, issue, branch, worktree path, allowed paths,
     read-first files, deliverable, required evidence, verification command or
     surface, stop condition, and return format.
   - For forked Codex worktree child lanes, the orchestrator MUST rename the
     child thread after setup with `set_thread_title` when available, using a
     title that clearly includes the project, issue number, and lane purpose.
   - Tell child implementation threads to create or maintain their own real
     goal state with `create_goal`, `get_goal`, and `update_goal` when
     available; keep real todo/plan state current with `update_plan` or the
     available todo surface; use required multi-agent execution for
     independent research, disjoint implementation, QA, verification, review,
     review-feedback validation, or separable non-trivial subtasks; and report
     actual tool usage, concrete not-useful rationale, or unavailable-tool
     fallbacks.
   - Tell child implementation threads and exploration agents to use Codexy
     `codegraph` MCP for code exploration when available, with ordinary file
     reads as confirmation before edits.
   - Tell child implementation threads to use Codexy `lsp` for language-aware
     code edits when a matching server is registered and callable, or return
     unavailable/not applicable evidence from `lsp_status`.
   - Tell each non-trivial child implementation thread to run the packaged
     Codexy reviewer agent before handoff, PR readiness, completion, or parent
     acceptance, and include current-diff reviewer findings or approval in its
     return evidence.
   - For non-trivial code, validator, harness, or workflow-rule lanes, require
     `scripts/validate-plugin-config --check-touched-loc --base-ref <base>`
     output before handoff or PR readiness. Over-250 LOC implementation or
     test-harness files must be fixed unless the tracked Codexy LOC exception
     mechanism names the file and rationale.
   - Require evidence, diffs, findings, or failed assumptions; do not accept
     acknowledgements as proof.
   - For Codex worktree thread lanes, state that the child owns implementation
     edits and review-response fixes for that lane.
4. Integrate:
   - Re-read files and outputs before trusting child results.
   - Preserve user changes and unrelated work.
   - Resolve cross-lane conflicts in the orchestrator thread.
   - Route child-owned review feedback back to the owning child thread instead
     of patching it in the orchestrator thread.
   - If the child owner stops responding, stop and report the blocked state with
     the PR head, owner, last contact, and required evidence. Do not recover by
     patching the child-owned branch unless a maintainer explicitly reassigns
     implementation ownership to the parent.
   - If parent-authored draft edits are discovered for a child-owned lane, stop
     parent implementation, preserve or revert only as needed to protect user
     work, and route the draft diff to the child as input evidence.
   - If a child lane has bundled independent outcomes, stop the bundled lane,
     preserve and report its draft state, and split the work into atomic
     issues, child threads, worktrees, branches, and PRs before continuing.
   - While child work, worktree setup, Codex review, or asynchronous tools are
     pending, keep polling and updating the plan instead of marking the goal
     blocked.
5. Verify:
   - Run local checks in the owning worktree.
   - Drive external surfaces directly when the task changes GitHub, browser,
     CLI, desktop, plugin, marketplace, or repository settings behavior.
   - Keep evidence tied to the exact commit, PR head, file state, or runtime
     surface being claimed.
6. Finish:
   - Confirm no running sessions, open child lanes, untracked required files, or
     unverified claims remain.
   - Confirm no final-answer or handoff artifact claims completion while a
     matching clean PR remains open unless the maintainer explicitly requested
     stop, wait, draft-only, or leave-open behavior.
   - Report what changed, what proved it, what was not run, and remaining risk.

## Multi-Agent Dispatch Template

```text
Lane goal / success criteria:
Task classification:
Atomic lane:
Issue:
Branch:
Worktree path:
Allowed paths:
Read first:
Deliverable:
Verification:
Required evidence:
Classification evidence:
Review feedback route:
Parent verification:
Return evidence:
  - Goal tool usage or unavailable-goal-tool fallback
  - Todo/plan tool usage or unavailable-todo-tool fallback
  - Touched implementation-file LOC gate output or not-applicable rationale
  - Codegraph findings and LSP status or unavailable/not-applicable evidence
  - Multi-agent usage for separable subtasks, or a concrete not-useful
    rationale tied to atomicity, tiny scope, or unavailable tooling
  - Packaged Codexy reviewer gate result for the current diff, exact head or
    file state, scope, verification outputs, and evidence
Child execution discipline:
Stop if:
```

## Codex Thread And Worktree Handoff

Use this for any lane that needs its own branch, PR, or long-running
implementation context:

```text
Issue:
Branch:
Worktree path:
Task classification:
First message:
Allowed files or paths:
Read first:
Acceptance criteria:
Required evidence:
Stop condition:
Parent verification:
Return format:
  - Include goal tool usage or unavailable-goal-tool fallback.
  - Include todo/plan tool usage or unavailable-todo-tool fallback.
  - Include multi-agent usage or a concrete not-useful/unavailable-tool
    rationale.
  - Include codegraph findings and LSP status or unavailable/not-applicable
    evidence for code-touching lanes.
  - Include touched implementation-file LOC gate output for non-trivial code,
    validator, harness, or workflow-rule lanes.
  - Include packaged Codexy reviewer gate findings or approval for the current
    diff, exact head or file state, scope, verification outputs, and evidence.
```

- Prefer Codex app thread tools such as `fork_thread` or `create_thread` with a
  `worktree` environment when they are available in the session.
- A child worktree thread should create or use exactly one task branch with the
  project branch prefix.
- The child thread must not merge, close issues, or claim final completion.
  It returns evidence and a commit-ready branch to the invoking orchestrator
  thread.
- The invoking Codex thread must not edit implementation files for this
  handoff before the child thread is created, forked, assigned, and given the
  stop condition. If accidental parent draft edits exist, include them as
  draft-diff input and stop parent implementation.
- The invoking Codex thread re-reads diffs, reruns required checks, handles PR
  review gates, merges through GitHub, deletes branches, and syncs main.

## Thread Tool Discovery Procedure

Use this before declaring Codex thread/worktree tooling unavailable, before
reporting a parent blocker caused by missing thread tools, or before routing a
child-owned implementation lane through another surface.

1. Search the actual callable tool surface for true Codex thread/worktree tool
   names and namespaces. Include exact and broad terms such as
   `codex_app create_thread fork_thread list_threads read_thread
   send_message_to_thread set_thread_title`, `thread/start`, `turn/start`,
   `Thread Coordination`, `Codex managed worktree`, `worktree`, and
   `child thread`.
2. Separately record `tool_search` results and any actual thread-event
   evidence. A tool_search mismatch is an exposure/discovery defect when it
   misses the thread namespace while another real surface produces
   `thread/start` and `turn/start` events. Do not report the tools absent or
   block the lane only because `tool_search` missed them when those events
   exist.
   If `tool_search` or the visible tool surface discovers a Codex app thread
   tool such as `read_thread` or `set_thread_title`, but invocation fails with
   `No handler registered for tool: ...`, record both surfaces as a
   dogfooding/tool-exposure defect: the discovered or listed tool metadata and
   the runtime missing-handler evidence. Do not treat handler-missing evidence
   as an ordinary unavailable-tool fallback.
3. Treat app-server-observed `thread/start` and `turn/start` evidence from a
   freshly created child lane as proof that a real Codex thread started. Record
   the observed event source, issue or lane, branch or worktree target when
   available, and the active owner identity. This is evidence of the thread
   surface; it is not permission to replace thread tooling with generic
   app-server or CLI commands.
4. Subagents are not child-owned implementation owners. `spawn_agent`,
   `multi_agent_v1`, specialist agents, or other subagent tools may help with
   bounded research or review, but they do not satisfy the clean Codex
   thread/worktree requirement for an issue-sized implementation branch or PR.
5. Do not use `codex exec`, `codex fork`, or `codex app-server` commands as
   fallback substitutes for true thread/worktree tools. In particular, an
   app-server command that only sends a message is not equivalent to a thread
   creation tool with project, worktree, starting ref, branch, and owner
   targeting.
6. If no real thread surface is found after the searches above, record an
   exposure/discovery defect with both surfaces: the expected/registered or
   historically observed thread tool names, the exact `tool_search` query and
   result categories, and any missing prompt/tool namespace evidence. Stop
   parent implementation routing until a real thread owner is assigned or a
   maintainer explicitly changes the lane requirement.
7. If a real thread surface is already producing `thread/start` or
   `turn/start`, keep the lane active and route through that surface instead
   of immediately reporting a blocker. False blockers caused by `tool_search`
   alone are workflow defects.

## Codex App Worktree Creation Preflight

Use this when calling Codex app thread/worktree tools such as `fork_thread` or
`create_thread` with a `worktree` environment.

- Preflight branch names with local Git before requesting the app worktree:
  `git check-ref-format --branch <branch>`, `git rev-parse --verify <branch>`,
  and `git rev-parse --verify origin/<branch>` as applicable.
- Do not pass a non-existing new branch as
  `startingState.type="branch"` / `branchName=<new-branch>`. Treat
  `startingState.type="branch"` as an existing ref selector unless the tool
  documentation or current successful evidence proves it creates new branches.
  For a new lane branch, create the branch with the tool's explicit new-branch
  mode if available, or create it with Git in the owning worktree after the
  worktree starts from a known existing ref.
- If `git check-ref-format --branch <branch>` succeeds but the Codex app
  reports `fatal: invalid reference: <branch>` during `Creating worktree`, do
  not "fix" the branch spelling or blindly retry. First check whether the
  branch exists locally or remotely; if it does not, record the likely
  existing-ref expectation and retry only with a valid starting ref or an
  explicit new-branch creation path.
- Waiting for pending worktree setup is an active orchestration state. Poll or
  wait for the pending result, keep the plan item in progress, and do not judge
  the lane failed merely because setup has not completed quickly.
- Keep exactly one active owner for each issue-sized lane. Before retrying or
  reassigning after pending or failed setup, list current child threads,
  pending worktrees, branches, and worktree paths when the tools expose them.
  Stop, archive, remove, or explicitly mark duplicate owners inactive before
  creating another owner for the same lane.
- Handoff evidence for Codex app worktree setup must include the starting ref,
  branch preflight result, pending/final worktree result, active owner identity,
  and any duplicate-owner cleanup.

## Worktree Rules

- One issue-sized outcome per branch.
- One branch per pull request.
- One independent requested outcome per child lane unless a maintainer
  explicitly scoped multiple outcomes as one atomic lane before implementation.
- Worktree-based implementation lanes require a Codex thread when thread tools
  are available.
- Worktree-based implementation lanes require lane ownership before edits:
  parent coordination first, child implementation second.
- Shared files must have a named owner before parallel edits begin.
- Never merge child work locally as a substitute for the repository PR flow.
- After merge, synchronize the main worktree before starting dependent work.

## Completion Guard

Do not mark a plan step complete until its evidence has been inspected by the
orchestrator. Use `update_goal` only when that tool is available, an active or
user-requested goal exists, and every explicit requirement has current,
matching proof. Otherwise, report the same completion audit textually without
inventing unavailable or unrequested goal-tool calls.

Do not call `update_goal(status="blocked")` merely because Codex review is
pending, a child thread is still working, worktree/thread setup is queued, or an
asynchronous tool has not returned yet. Those are active waiting states. Poll,
route feedback to the owning child thread, request fresh review on new PR heads,
and keep integrating until the merge loop completes or a repeated true impasse
requires user input or an external state change.

## Failure Modes

- Treating an `eyes` reaction, child acknowledgement, or green test as complete
  proof.
- Marking a goal blocked because a review, child thread, queued worktree/thread,
  or asynchronous tool is still pending.
- Treating expected or registered MCPs as ordinary unavailable tools when the
  callable Codex surface does not expose them, instead of following root
  `AGENTS.md` dogfooding policy.
- Retrying Codex app worktree setup after `fatal: invalid reference` by
  creating another active owner without checking whether
  `startingState.type="branch"` expected an existing ref.
- Sending duplicate `@codex review` requests while an existing request already
  has `eyes` for the same PR head. Keep polling and waiting; if the request is
  unusually stale, record that status and escalate with a distinct rationale
  instead of repeated blind requests.
- Leaving multiple forked child worktree threads with inherited parent titles
  when `set_thread_title` is available.
- Starting parent implementation patches for a lane that needs its own child
  thread, worktree, branch, or PR, then delegating only after files changed.
- Continuing parent implementation after discovering accidental draft edits for
  a child-owned lane instead of handing the draft diff to the child thread.
- Letting a child lane expand scope or edit shared files without ownership.
- Letting a child lane bundle independent requested outcomes instead of
  stopping, preserving draft state, and splitting the work into atomic issues,
  threads, worktrees, branches, and PRs.
- Letting a child implementation thread skip goal, todo/plan, or required
  situational multi-agent discipline without saying which tool was unavailable
  or giving a concrete not-useful rationale tied to atomicity, tiny scope, or
  the absence of separable work.
- Treating parent-only readthrough, arbitrary reviewer agents, generic review
  roles, or stale reviewer output as the packaged Codexy reviewer gate for the
  current diff and evidence.
- Fixing a child-owned PR's review feedback in the parent/orchestrator thread
  instead of routing it back to the owning child thread.
- Keeping work in a broad umbrella branch instead of issue-sized PRs.
- Reporting completion while review comments, open threads, or stale PR heads
  remain unresolved.
