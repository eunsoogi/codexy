---
name: codex-orchestration
description: Use when coordinating Codex plugin calls, long-running goals, issue-sized decomposition, multi-agent or multi-thread execution, worktrees, todo/update_plan tracking, and orchestrator-led implementation loops.
---

# Codex Orchestration

## Purpose

Run the current Codex thread as the orchestrator for goal-oriented work. The
orchestrator owns intent, decomposition, routing, evidence integration, and the
final completion claim. Subagents, extra threads, and worktrees own bounded
atomic units only.

## Parent And Child Thread Boundary

- The plugin-invoking Codex thread is the orchestrator. It creates or confirms
  issues, assigns branches, delegates lanes, opens PRs when appropriate,
  requests Codex review, performs parent verification, coordinates squash merge,
  and syncs `main`.
- A child Codex worktree thread owns implementation edits, local verification,
  and review-response fixes for its assigned issue or lane.
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
- Worktree lanes must stay issue-sized and atomic. Do not bundle review
  response work from one lane into another lane.

## Child Execution Discipline

Child implementation threads assigned a non-trivial lane MUST run their own
execution loop instead of treating the parent handoff as permission for ad hoc
edits.

- Create or maintain a lane-specific goal when goal tooling is available. If
  goal tooling is unavailable, write a visible textual goal with success
  criteria and keep it current in status updates and handoff evidence.
- Maintain a visible todo or `update_plan` state for multi-step delegated work.
  Update statuses as work moves from discovery, to edit, to verification, to
  handoff.
- Use multi-agent decomposition when independent research, implementation,
  review, QA, or verification subtasks can safely proceed in parallel and the
  tool is available and useful.
- Atomic trivial child tasks may stay lightweight, but substantial delegated
  work MUST NOT proceed as untracked edits without goal and todo discipline.
- If a required execution tool is unavailable in the child thread, say so in
  the thread and use the closest available fallback. Do not silently skip the
  discipline.
- The parent/orchestrator monitors evidence and merge gates. The child owns
  the implementation loop, local verification, and review-response fixes for
  its lane until the stop condition is met.

## Required Control Plane

- Establish the goal before implementation. If `create_goal` is available and
  the user explicitly asks for goal tracking, use it. If goal tools are not
  available, keep a visible `Goal` note in the thread with success criteria and
  update it textually as evidence changes.
- Maintain a visible todo list with `update_plan` for any non-trivial task.
- Decompose broad work into issue-sized atomic units before editing.
- Use multi-agent dispatch for independent research, implementation, QA,
  review, or release lanes when the tool is available.
- Use multi-thread or worktree decomposition when lanes can proceed
  independently, touch separate ownership areas, or need separate PRs.
- Keep the current thread as the orchestrator. It integrates child results,
  resolves conflicts, verifies final behavior, and decides whether work is
  complete.

## Orchestration Loop

1. Intake:
   - Read the latest user request, repository instructions, active issue, and
     relevant local skills.
   - Separate hard requirements, preferences, assumptions, and non-goals.
   - Identify the observable surface that proves the request worked.
2. Plan:
   - Create a short `update_plan` with atomic outcomes.
   - Mark exactly one step `in_progress`.
   - Split unrelated outcomes into separate issues or worktrees.
3. Dispatch:
   - Start subagents only for independent lanes.
   - Give each lane an assignment, allowed paths, required reads, deliverable,
     verification command or surface, and stop condition.
   - Tell child implementation threads to create or maintain their own goal,
     keep todo/plan state current, use useful multi-agent decomposition, and
     report unavailable-tool fallbacks.
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
Goal:
Atomic lane:
Allowed paths:
Read first:
Deliverable:
Verification:
Review feedback route:
Parent verification:
Return evidence:
Child execution discipline:
Stop if:
```

## Worktree Rules

- One issue-sized outcome per branch.
- One branch per pull request.
- Shared files must have a named owner before parallel edits begin.
- Never merge child work locally as a substitute for the repository PR flow.
- After merge, synchronize the main worktree before starting dependent work.

## Completion Guard

Do not mark a plan step complete until its evidence has been inspected by the
orchestrator. Use `update_goal` only when that tool is available, an active or
user-requested goal exists, and every explicit requirement has current,
matching proof. Otherwise, report the same completion audit textually without
inventing unavailable or unrequested goal-tool calls.

## Failure Modes

- Treating an `eyes` reaction, child acknowledgement, or green test as complete
  proof.
- Letting a child lane expand scope or edit shared files without ownership.
- Letting a child implementation thread skip goal, todo/plan, or useful
  multi-agent discipline without saying which tool was unavailable and which
  fallback was used.
- Fixing a child-owned PR's review feedback in the parent/orchestrator thread
  instead of routing it back to the owning child thread.
- Keeping work in a broad umbrella branch instead of issue-sized PRs.
- Reporting completion while review comments, open threads, or stale PR heads
  remain unresolved.
