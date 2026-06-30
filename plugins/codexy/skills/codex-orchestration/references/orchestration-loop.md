# Orchestration Loop

## Loop

1. Intake:
   - Run `$task-classification` before setup, delegation, implementation,
     validation, PR handling, review-response routing, or merge coordination.
   - Read the latest request, repository instructions, active issue, and
     relevant local skills.
   - Separate hard requirements, preferences, assumptions, and non-goals.
   - Identify the observable surface that proves the request worked.
   - Use Codexy `codegraph` MCP to map relevant files and neighbors when
     available.
2. Plan:
   - Create a short `update_plan` with atomic outcomes.
   - Mark exactly one step `in_progress`.
   - Carry classification evidence into the plan before branch, worktree,
     child-thread, implementation, PR, or review-response actions.
   - Split independent outcomes into separate issues and lanes unless a
     maintainer explicitly scopes them as one atomic lane.
   - Mark each lane as parent-owned or child-owned before any implementation
     patch is made.
3. Dispatch:
   - MUST NOT dispatch until classification proves lane type, owner, atomicity,
     required skills, required tools, and first allowed action.
   - Start specialist subagents only for bounded lanes that do not need their
     own branch or PR.
   - For issue-sized implementation lanes, start or fork a separate Codex
     thread in a worktree when the tool is available.
   - Complete lane assignment before implementation edits begin. A parent may
     prepare issue text, branch name, worktree path, and handoff text, but MUST
     NOT patch implementation files for the child-owned lane.
   - Give each lane an assignment, issue, branch, worktree path, allowed paths,
     read-first files, deliverable, required evidence, verification command or
     surface, stop condition, and return format.
4. Integrate:
   - Re-read files and outputs before trusting child results.
   - Preserve user changes and unrelated work.
   - Resolve cross-lane conflicts in the orchestrator thread.
   - Route child-owned review feedback back to the owning child thread.
   - If the child owner stops responding, stop and report the PR head, owner,
     last contact, and required evidence. MUST NOT recover by patching the branch
     unless a maintainer explicitly reassigns implementation ownership.
5. Verify:
   - Run local checks in the owning worktree.
   - Drive external surfaces directly when the task changes GitHub, browser,
     CLI, desktop, plugin, marketplace, or repository settings behavior.
   - Keep evidence tied to the exact commit, PR head, file state, or runtime
     surface being claimed.
6. Finish:
   - Confirm no running sessions, open child lanes, untracked required files,
     or unverified claims remain.
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
  - Include codegraph findings and LSP status or unavailable/not applicable
    evidence for code-touching lanes.
  - Include touched implementation-file LOC gate output for non-trivial code,
    validator, harness, or workflow-rule lanes.
  - Include packaged Codexy reviewer gate findings or approval for the current
    diff, exact head or file state, scope, verification outputs, and evidence.
```

The child thread MUST NOT merge, close issues, or claim final completion. It
returns evidence and a commit-ready branch to the invoking orchestrator thread.
