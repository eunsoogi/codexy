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

Codexy ships specialist agent definitions as plugin-packaged TOML files at
`plugins/codexy/agents/<name>.toml`, with discovery metadata in
`plugins/codexy/agents/catalog.toml`. Keep one specialist agent per file.
`plugins/codexy/agents/openai.yaml` is the plugin invocation interface, not a
specialist worker. Do not treat `plugins/codexy/.codex/agents` as installed
custom agents: Codex discovers native custom agents from the active project
`.codex/agents` or `~/.codex/agents`, not from an installed plugin's internal
`.codex/agents` directory.

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

## Child Thread Titles

- After a forked Codex worktree child thread finishes setup and a thread id is
  available, the orchestrator should rename it with `set_thread_title` when the
  tool is available.
- Use a title that includes the project, issue number, and lane purpose, such
  as `Codexy #52 refactoring skill agent lane`.
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
  `plugins/codexy/agents/reviewer.toml` against the current lane diff, exact
  head or file state, lane scope, verification outputs, and available evidence.
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
  thread when the lane does not need its own branch or PR. Multi-agent use is
  required when independent research questions, disjoint implementation slices,
  parallel QA or verification, review gates, review-feedback validation, or
  separable subtasks in a non-trivial atomic lane can be isolated. Use the
  packaged specialist agent files and lightweight catalog metadata as routing
  context; do not claim those packaged agents are native Codex custom agents
  unless they have been projected into the active project or user custom-agent
  directory by a supported workflow.
- For repository code exploration, route threads and agents through the
  packaged Codexy `codegraph` MCP when it is available before falling back to
  ad hoc text search. Use codegraph output to identify files, import edges,
  and nearby implementation surfaces, then confirm with direct file reads
  before editing.
- End every non-trivial atomic unit with the packaged Codexy reviewer agent
  from `plugins/codexy/agents/reviewer.toml`. The reviewer gate belongs inside
  the owning thread or child thread for that atomic unit and must review the
  current diff, exact head or file state, lane scope, verification outputs,
  and evidence before handoff, PR readiness, completion, or parent acceptance.
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
   - Read the latest user request, repository instructions, active issue, and
     relevant local skills.
   - Separate hard requirements, preferences, assumptions, and non-goals.
   - Identify the observable surface that proves the request worked.
   - For codebase discovery, use the Codexy `codegraph` MCP to map relevant
     files and neighbors when available.
2. Plan:
   - Create a short `update_plan` with atomic outcomes.
   - Mark exactly one step `in_progress`.
   - Split independent outcomes into separate issues and, when implementation
     can proceed independently, separate Codex thread/worktree lanes. Treat
     bundling independent requested outcomes as a workflow violation unless a
     maintainer explicitly scopes them as one atomic lane.
   - Mark each lane as parent-owned or child-owned before any implementation
     patch is made.
3. Dispatch:
   - Start specialist subagents only for bounded lanes that do not need their
     own branch or PR.
   - For issue-sized implementation lanes, start or fork a separate Codex
     thread in a worktree when the tool is available. Fall back to manual
     `git worktree` only when thread tooling is unavailable, and record why.
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
   - For forked Codex worktree child lanes, rename the child thread after
     setup with `set_thread_title` when available, using a project, issue
     number, and lane purpose title.
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
   - Tell each non-trivial child implementation thread to run the packaged
     Codexy reviewer agent before handoff, PR readiness, completion, or parent
     acceptance, and include current-diff reviewer findings or approval in its
     return evidence.
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
   - Report what changed, what proved it, what was not run, and remaining risk.

## Multi-Agent Dispatch Template

```text
Lane goal / success criteria:
Atomic lane:
Issue:
Branch:
Worktree path:
Allowed paths:
Read first:
Deliverable:
Verification:
Required evidence:
Review feedback route:
Parent verification:
Return evidence:
  - Goal tool usage or unavailable-goal-tool fallback
  - Todo/plan tool usage or unavailable-todo-tool fallback
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
