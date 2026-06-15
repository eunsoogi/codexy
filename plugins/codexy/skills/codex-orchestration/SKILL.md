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

Codexy ships specialist role definitions as plugin-packaged metadata at
`plugins/codexy/agents/roles.toml`. Do not treat
`plugins/codexy/.codex/agents` as installed custom agents: Codex discovers
native custom agents from the active project `.codex/agents` or
`~/.codex/agents`, not from an installed plugin's internal `.codex/agents`
directory.

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

## Required Control Plane

- Establish the goal before implementation. If `create_goal` is available and
  the user explicitly asks for goal tracking, use it. If goal tools are not
  available, keep a visible `Goal` note in the thread with success criteria and
  update it textually as evidence changes.
- Maintain a visible todo list with `update_plan` for any non-trivial task.
- Decompose broad work into issue-sized atomic units before editing.
- Use multi-agent dispatch for bounded specialist help inside the current
  thread when the lane does not need its own branch or PR. Use the packaged
  specialist role catalog as routing context; do not claim those roles are
  native Codex custom agents unless they have been projected into the active
  project or user custom-agent directory by a supported workflow.
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
2. Plan:
   - Create a short `update_plan` with atomic outcomes.
   - Mark exactly one step `in_progress`.
   - Split unrelated outcomes into separate issues and, when implementation
     can proceed independently, separate Codex thread/worktree lanes.
3. Dispatch:
   - Start specialist subagents only for bounded lanes that do not need their
     own branch or PR.
   - For issue-sized implementation lanes, start or fork a separate Codex
     thread in a worktree when the tool is available. Fall back to manual
     `git worktree` only when thread tooling is unavailable, and record why.
   - Give each lane an assignment, issue, branch, worktree path, allowed paths,
     read-first files, deliverable, required evidence, verification command or
     surface, stop condition, and return format.
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
```

- Prefer Codex app thread tools such as `fork_thread` or `create_thread` with a
  `worktree` environment when they are available in the session.
- A child worktree thread should create or use exactly one task branch with the
  project branch prefix.
- The child thread must not merge, close issues, or claim final completion.
  It returns evidence and a commit-ready branch to the invoking orchestrator
  thread.
- The invoking Codex thread re-reads diffs, reruns required checks, handles PR
  review gates, merges through GitHub, deletes branches, and syncs main.

## Worktree Rules

- One issue-sized outcome per branch.
- One branch per pull request.
- Worktree-based implementation lanes require a Codex thread when thread tools
  are available.
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
- Fixing a child-owned PR's review feedback in the parent/orchestrator thread
  instead of routing it back to the owning child thread.
- Keeping work in a broad umbrella branch instead of issue-sized PRs.
- Reporting completion while review comments, open threads, or stale PR heads
  remain unresolved.
