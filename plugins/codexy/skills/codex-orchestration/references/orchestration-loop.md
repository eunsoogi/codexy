# Orchestration Loop

Before any child-created issue mutation, MUST send the parent one canonical
machine-readable receipt, receive explicit approval, and pass
`--check-issue-intake`. MUST require the intake gate and explicit parent
approval before separate tracking.
Automatic creation MUST NOT be authorized. Unsupported synthetic or
same-class variants MUST remain handoff observations. MUST use typed decisions rather
than infer approval, support, ownership, necessity, or classification from
rationale wording.

## Loop

1. Intake:
   - MUST run `$task-classification` before setup, delegation, implementation,
     validation, PR handling, review-response routing, or merge coordination.
   - MUST read the latest request, repository instructions, active issue, and
     relevant local skills.
   - MUST separate hard requirements, preferences, assumptions, and non-goals.
   - MUST identify the observable surface that proves the request worked.
   - MUST use Codexy `codegraph` MCP to map relevant files and neighbors when
     available.
2. Plan:
   - MUST create a short `update_plan` with atomic outcomes.
   - MUST mark exactly one step `in_progress`.
   - MUST carry classification evidence into the plan before branch, worktree,
     child-thread, implementation, PR, or review-response actions.
   - MUST split independent outcomes into separate issues and lanes unless a
     maintainer explicitly scopes them as one atomic lane.
   - MUST mark each lane as parent-owned or child-owned before any implementation
     patch is made.
3. Dispatch:
   - MUST NOT dispatch until classification proves lane type, owner, atomicity,
     required skills, required tools, and first allowed action.
   - The root orchestrator MUST start specialist subagents only for bounded lanes without their own
     branch or PR.
   - Every helper or Sentinel assignment MUST include the nonrecursive delegation prohibition.
     `MUST NOT spawn, delegate to, or create any additional agent, helper, reviewer, task, or thread.`
   - For bounded helper work, the owning thread MUST route to the packaged
     Codexy specialist whose stated scope clearly matches the task, or record a
     concrete skip rationale. It MUST NOT count that specialist as the Codex
     child-thread/worktree owner for an issue-sized implementation lane.
   - For issue-sized implementation lanes, the root orchestrator MUST start or fork a separate Codex
     thread in a worktree when the tool is available.
   - MUST complete lane assignment before implementation edits begin. A parent may
     prepare issue text, branch name, worktree path, and handoff text, but MUST
     NOT patch implementation files for the child-owned lane.
   - MUST give each lane an assignment, issue, branch, worktree path, allowed paths,
     read-first files, deliverable, required evidence, verification command or
     surface, stop condition, and return format.
4. Integrate:
   - MUST re-read files and outputs before trusting child results.
   - MUST preserve user changes and unrelated work.
   - MUST resolve cross-lane conflicts in the orchestrator thread.
   - MUST route child-owned review feedback back to the owning child thread.
   - If the child owner stops responding, MUST stop and report the PR head, owner,
     last contact, and required evidence. MUST NOT recover by patching the branch
     unless a maintainer explicitly reassigns implementation ownership.
5. MUST verify:
   - MUST run local checks in the owning worktree.
   - MUST drive external surfaces directly when the task changes GitHub, browser,
     CLI, desktop, plugin, marketplace, or repository settings behavior.
   - MUST keep evidence tied to the exact commit, PR head, file state, or runtime
     surface being claimed.
6. Finish:
   - MUST confirm no running sessions, open child lanes, untracked required files,
     or unverified claims remain.
   - MUST confirm no final-answer or handoff artifact claims completion while a
     matching clean PR remains open unless the maintainer explicitly requested
     stop, wait, draft-only, or leave-open behavior.
   - MUST report what changed, what proved it, what was not run, and remaining risk.

## Failure Modes

- Starting setup, delegation, implementation, validation, PR handling, review
  response, or merge coordination before `$task-classification`.
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

## Multi-Agent Dispatch Template

```text
Lane goal / success criteria:
| Task classification | Decision |
| --- | --- |
| Lane type | |
| Secondary surfaces | |
| Owner decision | |
| Atomic scope | |
| Required skills | |
| Required tools/evidence | |
| First allowed action | |
| Stop/blocker | |
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

MUST use this for any lane that needs its own branch, PR, or long-running
implementation context:

```text
Issue:
Branch:
Worktree path:
| Task classification | Decision |
| --- | --- |
| Lane type | |
| Secondary surfaces | |
| Owner decision | |
| Atomic scope | |
| Required skills | |
| Required tools/evidence | |
| First allowed action | |
| Stop/blocker | |
First message:
Allowed files or paths:
Read first:
Acceptance criteria:
Required evidence:
Stop condition:
Parent verification:
Return format:
  - MUST include goal tool usage or unavailable-goal-tool fallback.
  - MUST include todo/plan tool usage or unavailable-todo-tool fallback.
  - MUST include multi-agent usage or a concrete not-useful/unavailable-tool
    rationale.
  - MUST include codegraph findings and LSP status or unavailable/not applicable
    evidence for code-touching lanes.
  - MUST include touched implementation-file LOC gate output for non-trivial code,
    validator, harness, or workflow-rule lanes.
  - MUST include packaged Codexy reviewer gate findings or approval for the current
    diff, exact head or file state, scope, verification outputs, and evidence.
```

The child thread MUST NOT merge, close issues, or claim final completion. It
returns evidence and a commit-ready branch to the invoking orchestrator thread.
