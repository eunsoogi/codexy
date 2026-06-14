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

## Required Control Plane

- Establish the goal before implementation. Use `create_goal` when the user
  explicitly asks for goal tracking; otherwise restate the objective and success
  criteria in the thread.
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
   - Require evidence, diffs, findings, or failed assumptions; do not accept
     acknowledgements as proof.
4. Integrate:
   - Re-read files and outputs before trusting child results.
   - Preserve user changes and unrelated work.
   - Resolve cross-lane conflicts in the orchestrator thread.
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
Return evidence:
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
orchestrator. Do not mark a goal complete until every explicit requirement has
current, matching proof.

## Failure Modes

- Treating an `eyes` reaction, child acknowledgement, or green test as complete
  proof.
- Letting a child lane expand scope or edit shared files without ownership.
- Keeping work in a broad umbrella branch instead of issue-sized PRs.
- Reporting completion while review comments, open threads, or stale PR heads
  remain unresolved.
