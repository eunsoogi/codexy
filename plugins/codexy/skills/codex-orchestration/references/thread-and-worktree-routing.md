# Thread And Worktree Routing

## Thread Tool Discovery Procedure

MUST use this before declaring Codex thread/worktree tooling unavailable, before
reporting a parent blocker caused by missing thread tools, or before routing a
child-owned implementation lane through another surface.

1. MUST search the actual callable tool surface for true Codex thread/worktree tool
   names and namespaces. MUST include exact and broad terms such as
   `codex_app create_thread fork_thread list_threads read_thread
   send_message_to_thread set_thread_title`, `thread/start`, `turn/start`,
   `Thread Coordination`, `Codex managed worktree`, `worktree`, and
   `child thread`.
2. MUST separately record `tool_search` results and actual thread-event evidence. A
   `tool_search` mismatch is an exposure/discovery defect when it misses the
   thread namespace while another real surface produces `thread/start` and
   `turn/start` events.
3. If `tool_search` or the visible tool surface discovers a Codex app thread
   tool but invocation fails with `No handler registered for tool: ...`, record
   both the discovered metadata and runtime missing-handler evidence as a
   dogfooding/tool-exposure defect.
4. MUST treat app-server-observed `thread/start` and `turn/start` evidence from a
   freshly created child lane as proof that a real Codex thread started. This
   is not permission to replace thread tooling with generic app-server or CLI
   commands.
5. Subagents are not child-owned implementation owners. `spawn_agent`,
   `multi_agent_v1`, specialist agents, and other subagent tools may help with
   bounded research or review, but they MUST NOT be treated as clean Codex
   thread/worktree owners.
6. MUST NOT use `codex exec`, `codex fork`, or `codex app-server` commands as
   fallback substitutes for true thread/worktree tools.
7. If no real thread surface is found after discovery, MUST record an
   exposure/discovery defect with both expected/registered surfaces and the
   exact discovery evidence. MUST stop parent implementation routing until a real
   owner is assigned or a maintainer changes the lane requirement.

## Codex App Worktree Creation Preflight

MUST use this when calling Codex app thread/worktree tools such as `fork_thread` or
`create_thread` with a worktree environment.

- MUST inspect current child owner state before creating or resuming a child
  Codex thread. The preflight evidence MUST include the current active child
  Codex thread count and whether an existing thread owns the same issue or PR.
- MUST keep at most five active Codex app child threads at a time. MUST NOT call
  `create_thread`, `fork_thread`, or a child-thread resume/continue operation
  that would make six active Codex app child threads.
- If an existing usable thread already owns the same issue or PR, MUST reuse
  that owner thread or MUST continue that owner thread instead of creating a
  replacement. Replacement child threads MUST require inspected existing-owner
  evidence plus proof that the old owner is stopped, unusable, or explicitly
  superseded.
- Packaged specialist subagents are helper or reviewer roles and MUST NOT count
  toward the five active Codex app child-thread limit.
- MUST preflight branch names with local Git:

```sh
git check-ref-format --branch <branch>
git rev-parse --verify <branch>
git rev-parse --verify origin/<branch>
```

- MUST NOT pass a non-existing new branch as
  `startingState.type="branch"` / `branchName=<new-branch>`. MUST treat
  `startingState.type="branch"` as an existing ref selector unless the tool
  documentation or current successful evidence proves it creates new branches.
- If Codex app setup reports `fatal: invalid reference: <branch>` after
  branch-name validation succeeds, MUST check whether the branch exists locally or
  remotely before retrying.
- Waiting for pending worktree setup is an active orchestration state. Poll or
  wait for the pending result; MUST NOT judge the lane failed just because setup
  has not completed quickly.
- MUST keep exactly one active owner for each issue-sized lane. Before retrying or
  reassigning after pending or failed setup, list current child threads,
  pending worktrees, branches, and worktree paths when the tools expose them.
- Handoff evidence for Codex app worktree setup MUST include starting ref,
  branch preflight result, pending/final worktree result, active owner identity,
  and duplicate-owner cleanup if any.

## Child Thread Titles

- After a forked Codex worktree child thread finishes setup and a thread id is
  available, the orchestrator MUST rename it with `set_thread_title` when that
  tool is available.
- The child thread title MUST clearly include the project, issue number, and
  lane purpose, such as `Codexy #52 refactoring skill agent lane`.
- If title renaming is unavailable, mention that limitation in orchestration
  status or child handoff and continue the lane.
- Child thread title renaming is a clarity policy, not a merge blocker for
  otherwise complete implementation work.

## Worktree Rules

- One issue-sized outcome per branch.
- One branch per pull request.
- One independent requested outcome per child lane unless a maintainer
  explicitly scoped multiple outcomes as one atomic lane before implementation.
- Orchestrators MUST keep at most five Codex app child threads active
  concurrently for orchestrator-created or orchestrator-resumed child lanes.
- Existing issue or PR owner threads MUST be reused when present and usable;
  replacement owner threads MUST require old-owner stop, unusable, or
  supersession evidence.
- Worktree-based implementation lanes MUST require a Codex thread when thread tools
  are available.
- Worktree-based implementation lanes MUST require lane ownership before edits:
  parent coordination first, child implementation second.
- Shared files MUST have a named owner before parallel edits begin.
- MUST NOT merge child work locally as a substitute for the repository PR flow.
- After merge, synchronize the main worktree before starting dependent work.
