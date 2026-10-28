# Thread And Worktree Routing

## Thread Tool Discovery Procedure

MUST use this before declaring Codex thread/worktree tooling unavailable, before
reporting a parent blocker caused by missing thread tools, or before routing a
child-owned implementation lane through another surface.

1. MUST search the actual callable tool surface for true Codex thread/worktree
   tool names and namespaces. MUST include exact and broad terms such as
   `codex_app create_thread fork_thread list_threads read_thread send_message_to_thread set_thread_title`,
   `thread/start`, `turn/start`, `Thread Coordination`,
   `Codex managed worktree`, `worktree`, and `child thread`.
2. MUST separately record `tool_search` results and actual thread-event
   evidence. A `tool_search` mismatch is an exposure/discovery defect when it
   misses the thread namespace while another real surface produces
   `thread/start` and `turn/start` events.
3. If `tool_search` or the visible tool surface discovers a Codex app thread
   tool but invocation fails with `No handler registered for tool: ...`, record
   both the discovered metadata and runtime missing-handler evidence as a
   dogfooding/tool-exposure defect. This is not ordinary unavailable thread
   tooling: the handoff or status MUST name the exact discovered tool, the exact
   missing-handler error, the fallback route used to reach the intended child
   owner or reviewer or explicitly state that no fallback route was available,
   and the separate dogfood issue that tracks the exposure/handler mismatch.
4. MUST treat app-server-observed `thread/start` and `turn/start` evidence from
   a freshly created child lane as proof that a real Codex thread started. This
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
   exact discovery evidence. MUST stop parent implementation routing until a
   real owner is assigned or a maintainer changes the lane requirement.

## App Task Creation Route

`create_thread` creates a separate app task. An explicit scoped orchestration
assignment authorizes the required bounded child tasks and necessary messages
under the [authority contract](child-routing-policy.md), without per-call
approval. A branch/worktree need, task complexity, or vague coordination request
alone MUST NOT authorize creation. Actual callable host limits still apply.

Before calling `create_thread`, the Orchestrator MUST read back separately:

- the actual callable tool and its invocation contract;
- the explicit task request or scoped orchestration authority; and
- the current task and existing active owner for the same issue or lane.

The Orchestrator MUST verify its actual task CWD before child creation. If the
CWD is in a linked Git worktree or cannot be verified inside the saved project's
primary checkout, it MUST NOT call `create_thread`. A different controlling task
MUST use the supported handoff surface to move the Orchestrator to the primary
repository; `handoff_thread` cannot move its own caller. After handoff, the
controlling task MUST verify the destination CWD and primary checkout from
current task readback before the Orchestrator resumes child creation. An
unverified destination MUST keep the lane pending.

For a selected child-owned lane, the Orchestrator MUST select the assigned
recipient model and effort automatically and MUST NOT ask the user to choose or
reconfirm them. Every Worker `create_thread` call MUST include the assigned
`model` and `thinking` pair and request `target.type="project"` with a non-empty
`projectId` and `target.environment.type="worktree"` in that same call. Local or
projectless Worker tasks MUST be denied before mutation; a later
`create_worktree`, `fork_thread`, shell `git worktree add`, or detached
directory MUST NOT substitute for the initial app-managed task environment.
Read-only Git inspection and supported app-managed worktree cleanup remain
valid. If the host cannot accept the worktree environment or assigned pair, the
lane MUST remain pending and the exact incompatibility MUST be reported. The
Orchestrator MUST NOT switch models, implement in the parent, or create a
duplicate Worker. A separate Orchestrator task is distinct from Worker-only
admission and MUST use its own assigned pair when explicitly requested. Model
routing does not grant authority to create a separate task.

When no separate task was requested and the current task is already explicitly
assigned as the implementation owner for that lane, the current-task route MUST
continue under its native goal without calling `create_thread`. A direct
standalone implementation request remains a valid assignment. For a request to
orchestrate independent issues, the parent/coordinator role, active overall
goal, or presence of the current task MUST NOT substitute for a lane-specific
implementation assignment. This route MUST NOT resolve unclear topology or
silently replace a selected child-owned lane. If topology is unresolved, or no
active child already owns a selected child-owned lane and explicit task-creation
authority is unavailable or ambiguous, implementation MUST remain pending while
the parent reports the exact owner decision or authority needed. When an active
child already owns the lane, the parent MUST send correction instructions
through the supported task route without requiring authority to create another
task, and MUST NOT implement in the parent or create a duplicate owner. A GitHub
assignee, issue, branch, PR, or delegated prompt MUST NOT substitute for an
active-owner readback.

When separate task creation is authorized and no existing owner conflict
remains, the Orchestrator MUST use the actual `create_thread` tool and read back
the returned task identity, owner, project, actual CWD, starting HEAD, app
worktree environment, exposed permissions and model settings, and native goal
before execution. The requested target, checkout path, or attached artifact
alone does not prove the task environment. The call is non-blocking: a ready
`threadId`/`hostId` is an actual task identity, while a setup `clientThreadId`
is only a pending setup identity and MUST NOT be passed to tools that require
`threadId`.

Both routes MUST preserve the native goal and ordinary direct `wait_threads`
observation. A separately assigned Watcher remains scoped to its exact targets.
Neither route may replace these host surfaces with text, an app-server/CLI path,
a fake task, or a silent fallback. Source tests, configuration, and app-server
transcripts MUST NOT substitute for actual host task creation and ownership
readback. If a required host route is unsupported, the agent MUST report the
exact limitation and required action and distinguish preparation from execution.

## Codex App Worktree Creation Preflight

MUST use this when calling Codex app thread/worktree tools such as `fork_thread`
or `create_thread` with a worktree environment.

The supported `create_thread(worktree)` contract owns worktree-path allocation.
This project-side preflight preserves known ownership and collision safeguards;
it does not reserve host paths or prove atomic exclusion. MUST NOT treat a
worktree requirement as authority to create a separate app task, and MUST NOT
require an additional undocumented reservation API or implement a replacement
host allocator.

## Live Worktree Setup Preflight

MUST run this before creating, forking, reusing, or recycling a Codex app
worktree for a repository-owned task. This is an active-project fail-closed
diagnostic for known ownership and setup state.

1. MUST rebuild the reservation map from the active/waiting child ledger and
   every active or waiting specialist or Sentinel. Each entry MUST name the
   canonical worktree CWD, frozen HEAD, clean/index state, available lock
   evidence, referencing task ids, role, status, and explicit release/archive
   state. The preflight MUST also record the active child count and whether an
   existing task owns the same issue or PR.
2. When a candidate path is exposed before allocation, MUST compare it with
   every known reservation and exclude a dirty, locked, or colliding candidate.
   A stop record MUST include the reserved path, referencing task ids/statuses,
   expected and observed HEAD/clean state, available lock evidence, and the
   conflict. The parent MUST NOT create or fork on that path, retry the same
   path, unlock it, clean it, archive it, or recycle it. When the host chooses
   the path internally, MUST invoke the supported worktree tool after this
   project preflight; local checks MUST NOT be described as host-level atomicity
   proof.
3. MUST keep exactly one active owner for each issue-sized lane. Before creating
   or resuming a child, MUST reconcile reserved, setting-up, and active slots
   against the
   [supervised-child ceiling](parent-supervision.md#supervised-app-child-capacity).
   At capacity, MUST retain excess work in the parent backlog without a child
   thread or active child goal. Resource exhaustion, unresolved ownership,
   dirty/locked checkout collisions, or actual shared-resource conflicts MUST
   defer a lane. The per-call `wait_threads` limit of eight targets is separate
   from the supervised app-child ceiling; splitting calls MUST NOT bypass its
   slot count. Native `spawn_agent` concurrency follows its own host/tool limit.
   If setup returns a `clientThreadId` or `pendingWorktreeId`, MUST retain it as
   a pending setup identity, not as a `threadId`. MUST wait for an authoritative
   ready or failed setup result; an observation timeout or an omitted list
   result is not by itself failure. While pending, MUST search by the pending
   identity, branch, issue/PR, SHA, and available review-thread id. Only a
   surfaced thread with an active owner, an actionable setup failure, or a
   bounded `not-surfaced-after-bounded-wait` state may end a setup search; any
   retry or reassignment MUST name that state. Ending the search MUST NOT by
   itself release the reserved capacity.
4. When setup is ready, MUST verify the returned task identity, CWD, HEAD, clean
   state, and owner before implementation starts. If the result conflicts with
   the ownership map or exposes unexpected dirty/locked state, MUST stop the
   lane without destructive cleanup or a duplicate retry.
5. MUST inspect actual setup status, approval, or permission state only when the
   host exposes it. MUST NOT infer parent permission inheritance, invent a
   permission-setting API, bypass approval, or declare creation failed because a
   task is temporarily absent from a list view.

- The parent MUST dispatch dependency-ready same-wave work concurrently within
  the combined reserved/setup/active ceiling, even when owned paths overlap.
  MUST NOT scan or compare paths as a dispatch gate. MUST preserve reservations
  across turns; splitting `wait_threads` batches MUST NOT bypass capacity.
- If an existing usable thread already owns the same issue or PR, MUST reuse
  that owner thread or MUST continue that owner thread instead of creating a
  replacement. Replacement child threads MUST require inspected existing-owner
  evidence plus proof that the old owner is stopped, unusable, or explicitly
  superseded.
- Packaged specialist subagents are helper or reviewer roles and MUST NOT be
  treated as issue-sized implementation owners; their use does not change the
  exactly-one-owner rule for each issue-sized lane.
- MUST preflight branch names with local Git:

```sh
git check-ref-format --branch <branch>
git rev-parse --verify <branch>
git rev-parse --verify origin/<branch>
```

- MUST NOT pass a non-existing new branch as `startingState.type="branch"` /
  `branchName=<new-branch>`. MUST treat `startingState.type="branch"` as an
  existing ref selector unless the tool documentation or current successful
  evidence proves it creates new branches.
- MUST omit `startingState` when the default branch is intended. MUST supply a
  validated existing branch or ref only when the user requested a particular
  starting point; MUST NOT invent a non-existent branch selector.
- If Codex app setup reports `fatal: invalid reference: <branch>` after
  branch-name validation succeeds, MUST check whether the branch exists locally
  or remotely before retrying.
- Waiting for pending worktree setup is an active orchestration state. Poll or
  wait for the pending result; MUST NOT judge the lane failed just because setup
  has not completed quickly.
- A `pendingWorktreeId` MUST follow the same pending identity and retained-slot
  rules above. Retry or reassignment MUST additionally reconcile actual setup
  and owner state; a bounded search or timeout MUST NOT free a slot.
- Handoff evidence for Codex app worktree setup MUST include starting ref,
  branch preflight result, pending/final worktree result, active owner identity,
  and duplicate-owner cleanup if any.

Child titles MUST follow the [title policy](child-thread-titles.md).

## Worktree Rules

- One issue-sized outcome per branch.
- One branch per pull request.
- One independent requested outcome per child lane unless a maintainer
  explicitly scoped multiple outcomes as one atomic lane before implementation.
- Orchestrators MUST preserve one owner and isolated app-managed worktree per
  issue. Path overlap MUST NOT serialize ready lanes; actual dependencies,
  capacity, permissions, and shared-resource reservations remain binding.
- Existing issue or PR owner threads MUST be reused when present and usable;
  replacement owner threads MUST require old-owner stop, unusable, or
  supersession evidence.
- Worktree-based implementation lanes MUST require lane ownership before edits:
  parent coordination first, child implementation second.
- #1447 elevation MAY fork into the same directory, preserving branch, checkout,
  HEAD, index, and required artifacts. The source MUST remain owner through
  target read-only setup; only the parent MAY transfer ownership after supported
  retirement. MUST NOT create a worktree, copy a diff, recreate a branch, permit
  concurrent writers, or reuse/clean another lane's dirty or locked checkout.
- Real integration conflicts MUST be resolved before merge through the owning
  task's branch, preserving both changes and rerunning affected verification.
- MUST NOT merge child work locally as a substitute for the repository PR flow.
- After merge, synchronize the main worktree before starting dependent work.

## Completed Child Cleanup

MUST follow
[accepted-child cleanup](../../../../codexy-github/skills/git-workflow/references/local-git-and-branches.md#accepted-child-cleanup):
verify completion/acceptance and no outstanding work; preserve ignored data and
confirm `archive_worktree`'s recoverable snapshot before `set_thread_archived`;
read back both before safe local issue-branch removal. MUST preserve uncertain
resources and report the exact gap. Default/shared/remote branches stay out of
scope.
