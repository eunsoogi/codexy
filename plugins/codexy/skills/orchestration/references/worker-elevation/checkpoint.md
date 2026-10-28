# Worker source checkpoint for elevation

## Boundary

This procedure prepares an existing Luna/Max Worker for a later, separately
authorized elevation transition. The source remains the issue owner and keeps
its active goal and reservation. This procedure MUST NOT fork or create a task,
archive or replace the source, change its model or goal, move its worktree, or
enable a live elevation entrypoint.

Checkpoint preparation does not execute elevation. After separate #1447
authorization, the Orchestrator MUST continue through the
[transition procedure](transition.md) and recheck this checkpoint before
target activation.

Here, quiescence means observed absence of implementation writes and live
file-writing producers under a cooperative no-write instruction that persists
across continuation turns. Automatic reasoning and read-only turns may still
happen. The source task and goal are not terminal, and this procedure MUST NOT
claim otherwise.

The Orchestrator MUST use this only after rechecking the current #1445 decision,
the current #1446 maintainer direction, source owner, exact assignment, scope,
and parent instructions. If any have changed, the Orchestrator MUST stop and
reassess before continuing. It MUST preserve the same checkout and its staged,
unstaged, required untracked, and required ignored state. This procedure MUST
NOT require a clean checkout.

## Select supported native operations

The Orchestrator MUST use the task's actual callable surface and current
readbacks. In the observed local Codex surface, it can request a cooperative
freeze through the existing authorized source-message route, then use
`list_threads`, `read_thread`, and `wait_threads` to observe task status and
recent turns. `wait_threads` waits for completion or attention; a timeout or
`idle` label is not evidence that a writer stopped. `read_thread_terminal` reads
only the current desktop task's terminal output, not another task's process
state.

The source's `get_goal({})` reads only that source task's goal. The Orchestrator
MUST obtain a fresh source-side call and its actual result; the Orchestrator's
goal, a title, or a worker's paraphrase is not a substitute. `list_threads`
exposes task identity, host, status, project, and CWD, but not branch, HEAD,
dirty state, running processes, or queued prompts. `read_thread` can show recent
tool output, but it can be truncated or stale and is not a live process or queue
inventory.

The current callable surface has no standalone current-turn cancel, queued
continuation inspection, or automatic-continuation disable operation. A message
requesting a freeze is cooperative; it is not a hard stop. The no-write
instruction MUST remain in force across continuation turns until an explicitly
supported transfer or parent release. Automatic reasoning and read-only turns
may still happen while the source goal is active. This procedure MUST NOT claim
that continuation is disabled or that the source task is terminal. The native
`handoff_thread` call interrupts a running _other_ task before moving its Git
state between a checkout and worktree, and cannot move its caller. That move is
outside this same-worktree procedure and MUST NOT be used as a freeze
substitute. Archiving the source or changing its goal status is also outside
this procedure. Any terminal stop, archive, or release needed for a separate
terminal handoff remains governed by the existing
[goal-transition reporting contract](../goal-transition-reporting.md); this
checkpoint does not waive it.

The source MAY request termination of its own known `exec_command` sessions by
writing to their returned handles with `write_stdin`; it MUST retain the actual
terminal exit result. A write request without an exit result is not termination
proof. This does not establish control over detached or unknown processes, and
the exposed schema does not establish that one task can control another task's
session. The source MAY use `list_agents` and `interrupt_agent` only for exact
IDs in its own native agent tree. An interrupt returns the agent's prior status,
and a later tree readback establishes agent-level status only; neither proves
that a child command, tool session, or detached process has ended. The source
MUST retain an actual terminal result for each identified child writer and each
file-writing tool call, command, or session it started. A terminal agent status
or prior-status response does not replace the result for that writer or any
command or session it launched. If a writer or its file-writing operation has no
actual termination or completion result, its state remains unknown and the
Orchestrator MUST NOT advance the checkpoint. The Orchestrator MUST obtain this
source-side readback; its own agent tools cannot inspect or stop the source
task's agents. These controls do not stop the source task itself.

If the source's cooperative no-write instruction is absent or no longer
applicable, or any identified file-writing producer remains active, the source
is not quiescent. The Orchestrator MUST preserve ownership, MUST NOT create a
target, and MUST report the exact unsupported operation and remaining producers.
An ACK, a quiet terminal, an idle status, or an unchanged Git readback alone
MUST NOT be reported as proof of quiescence. Automatic reasoning or read-only
continuation may occur under the active goal and is not itself a failed stop.

## Prepare and verify the checkpoint

1. The Orchestrator MUST recheck the issue, PR, latest #1445 decision and its
   hard-work evidence and event, the current #1446 maintainer direction, the
   parent instructions that approved that decision, actual source task ID and
   host, source ownership, requested model/effort, and the source's exact active
   goal and objective. It MUST record effective model/effort only when the host
   exposes it; otherwise record it as unknown. If the decision, evidence, event,
   maintainer direction, or parent instructions have changed, the Orchestrator
   MUST stop preparation and update the existing decision before continuing.
2. The Orchestrator MUST ask the source to finish any current write-producing
   call, stop launching writes, and keep a cooperative no-write instruction in
   force across continuation turns until an explicitly supported transfer or
   parent release. The source MUST stop only its identified writer sessions and
   subagents through supported controls. The Orchestrator MUST reconcile
   app-child producers from the parent's owner record; `list_threads` returns
   pinned and recent tasks, not a complete writer roster. The Orchestrator MUST
   NOT treat an ACK, `idle` label, or timeout as proof that a writer stopped. It
   MUST observe the exact source thread with the latest `wait_threads` cursor,
   read relevant turn output when a result is missing, truncated, or
   contradictory, and obtain an actual completion or termination readback for
   each identified writer. The source task may remain active and continue
   reasoning, reporting, or authorized read-only work. The Orchestrator MUST
   record its observed status without calling it terminal and MUST preserve
   read-only CI and other external computation in their existing roles.
3. Before recording the checkpoint, the Orchestrator MUST reconcile the known
   producer roster and establish that no identified producer is currently
   writing the same files and that the source's cooperative no-write instruction
   remains in force. Automatic reasoning and read-only turns do not by
   themselves invalidate this state. The checkpoint is not a durable lock: the
   authorized #1447 owner MUST independently recheck the source, writers, and
   checkout immediately before any later transition. If that readback observes
   an implementation write, a new writer, or changed checkpoint state, the
   Orchestrator MUST invalidate the record and wait for a new matching readback.
4. After identified current writers have ended and the source's cooperative
   no-write instruction is in force, the Orchestrator MUST have the source read
   its own goal and repository state from the same task and checkout. It MUST
   capture at least:

   - issue and PR, actual parent/source IDs and host, source issue ownership and
     reservation, exact active goal status and objective, observed task status,
     and unmet acceptance criteria;
   - project, actual CWD/worktree, branch, and HEAD;
   - staged and unstaged paths, required untracked paths, required ignored
     artifacts, content fingerprints for each staged and unstaged diff, and
     fingerprints for every required untracked or ignored artifact;
   - index-lock state;
   - the revision associated with each verification result, selected reviewer,
     and any unresolved review findings;
   - the open failure episode and remaining repair budget; and
   - requested and observed effective model/effort, keeping unknown values
     explicit.

   The Orchestrator MUST put this information in the existing task handoff
   record. It MUST NOT create a snapshot store, journal, or new serialization
   format. It MUST NOT copy ignored file contents or secrets into public
   records. The handoff record MUST preserve the checkpoint metadata above. For
   each artifact, it MUST store the path, status, fingerprint, and relevant
   file-type information; it MUST NOT copy artifact contents or secrets.
5. From the source's same checkout, the Orchestrator MUST read back `pwd`,
   `git rev-parse --show-toplevel`, `git rev-parse HEAD`,
   `git branch --show-current`,
   `git status --porcelain=v2 --branch --ignored=matching`,
   `git diff --cached --name-status`, `git diff --name-status`, and
   `git ls-files --others --exclude-standard`, plus the required ignored paths
   from `git ls-files --others --ignored --exclude-standard`. It MUST hash the
   staged and unstaged patch contents with
   `git diff --cached --binary | git hash-object --stdin` and
   `git diff --binary | git hash-object --stdin`. For each required untracked or
   ignored file, it MUST record a content fingerprint from
   `git hash-object --no-filters -- <path>` without using `-w`; this reads the
   bytes but does not write a Git object. Directory artifacts MUST be
   fingerprinted as their required file paths and contents. If a required
   artifact cannot be safely enumerated or fingerprinted, its state is unknown
   and the Orchestrator MUST NOT hand off the checkpoint. For symlinks or other
   non-regular files, fingerprint their link target or relevant metadata with a
   read-only method; if that cannot be established, the state is unknown and
   handoff MUST stop. It MUST also check whether the path from
   `git rev-parse --git-path index.lock` exists. Record the paths, statuses, and
   fingerprints in the existing task handoff record without copying sensitive
   contents.
6. Immediately before handoff, the Orchestrator MUST reread the source goal and
   its status, source task ID and host, issue owner and reservation, project,
   actual CWD/worktree, branch, HEAD, staged/unstaged/untracked/required-ignored
   state, diff and artifact fingerprints, index-lock state, verification
   revisions, reviewer and unresolved findings, failure episode and remaining
   repair budget, current producer roster and writer results, the continuing
   no-write instruction, and parent instructions and compare them with the
   existing record. If a recorded value or fingerprint changed, a new writer
   appeared, or an implementation write occurred, the Orchestrator MUST
   invalidate the record and prepare it again. Automatic reasoning or a
   read-only turn alone does not invalidate the record. Only after the record
   matches, the Orchestrator MUST provide it to the already authorized #1447
   owner through the existing information handoff route. This is not an
   ownership transfer: the source keeps the issue and reservation until an
   explicitly supported transfer. This preparation MUST NOT create, fork, or
   start a target task. The checkpoint is not a terminal stop or lock; #1447
   MUST perform its own immediate readback before proceeding. It MUST NOT
   commit, stash, reset, clean, or move the worktree to make the record match.

The checkpoint records a later transition's required source and checkout state
only when every listed field matches, each identified current writer has an
actual termination or completion readback, and the source's cooperative no-write
instruction remains in force. Automatic reasoning and read-only turns may still
happen; the checkpoint MUST NOT be described as a terminal task state or a lock
that prevents future continuation. Any new writer, implementation write, or
changed checkpoint field invalidates it. If a current writer or required field
cannot be established, the Orchestrator MUST keep the original source as owner
and report the verified portion plus the precise unsupported or changing state.
This procedure MUST NOT create the target in advance.
