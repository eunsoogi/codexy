# Executable-checkout recovery

This reference is part of [goal-lifecycle](../SKILL.md). It defines same-scope
recovery of an unfinished blocked assignment. It MUST NOT be used to authorize a
new task, clear a goal, or perform issue work before recovery acceptance.

## Authority and safety boundary

When the original assignment still authorizes the same objective and scope, that
authority carries through its recovery. A blocked state alone MUST NOT require
another user confirmation. An explicit user stop, revoked authority, changed
scope, missing original assignment, or actual host permission denial MUST stop
the sequence; preserve the source owner and report the exact reason. Host goal
and tool contracts remain authoritative.

An unfinished blocked goal MUST NOT be marked complete, cleared, or replaced to
obtain recovery. Recovery preparation is limited to preserving necessary owned
work, preparing one continuation, establishing its supported checkout
association, and checking the intended environment. Issue implementation and
ordinary issue verification remain stopped until the post-archive acceptance
checks below succeed.

Pre-archive attachment and process checks establish only the continuation's
current association and execution. They do not prove that source archival will
preserve the checkout. Step 5 requires a supported preservation route for the
exact source and continuation topology before the owner archives the source.

Recovery does not answer or clear an existing user-input gate. If the source has
an unanswered material user question, the owner MUST preserve that exact
question and its decision owner under the
[blocked-goal user-decision gate](../../orchestration/references/goal-transition-reporting.md#blocked-goal-user-decision-gate).
A fork, active-goal readback, parent report, or archive MUST NOT count as the
answer. Recovery preparation MAY proceed under the existing assignment, but
issue work that depends on the missing input MUST remain stopped until the
decision owner supplies it.

The
[official worktree guide](https://developers.openai.com/codex/app/worktrees#worktree-cleanup)
says Codex-managed worktrees can be deleted when their associated chat is
archived and that a snapshot is saved before deletion. A saved snapshot is
recoverable state, not proof that the same checkout remains executable.

## Source preflight and owned work

Before forking or archiving, the owner MUST capture:

- The exact blocked `get_goal` result, source task and authenticated parent,
  original authorized objective and scope, and current ownership.
- The exact recovery blocker and any unresolved user-input gate from the source.
  The owner MUST carry it into the continuation and MUST NOT treat recovery as
  answering it.
- Requested model and reasoning effort from the source assignment or creation
  record, separately from effective settings that the host exposes. For each
  continuation field, the owner MUST use an exposed effective value or its exact
  requested value. If either value is unavailable from both sources, it MUST
  stop before forking.
- A successful read-only process at the exact assigned CWD, including canonical
  repository root, absolute Git directory and common directory, HEAD,
  branch/detached state, staged and unstaged changes, tracked and untracked
  files, and Git worktree inventory.
- The native source task/worktree association and exposed attachment identity,
  including an explicit absent result. A saved CWD, Git registration, and native
  association are separate evidence.
- Required ignored data and any necessary owned changes, without disclosing
  secrets. The owner MUST preserve only necessary in-scope work, keep required
  ignored files in their authorized location, and MUST NOT stage or disclose
  credential values. It MUST resolve unexpected data or unclear ownership before
  continuing.
- Confirmation that no source or continuation process can write implementation
  changes during recovery preparation. The owner MUST NOT continue if this
  cannot be confirmed.

A scoped commit or stash MAY preserve selected changes. The owner MUST preserve
required ignored data separately. After any preservation write, the owner MUST
recapture the exact-CWD Git and file state and use that readback as the
preserved baseline. It MUST record how each necessary change and ignored file
can be verified from the continuation. Neither durable change preservation nor
an unchanged HEAD proves that the continuation checkout survives source
archival.

## Ordered blocked-goal procedure

1. The recovery owner MUST confirm that the original assignment still authorizes
   the same objective and scope. It MUST NOT request confirmation solely because
   the goal is blocked. If the authorization or scope is absent, stopped,
   revoked, or changed, it MUST preserve the source and stop.

2. The recovery owner MUST create exactly one same-directory fork with the
   supported `fork_thread` operation. It MUST NOT create a competing branch,
   worktree, replacement task, or second fork. If the fork fails or does not
   identify exactly one continuation, it MUST preserve the exact result and stop
   with the source as owner.

3. The recovery owner MUST send the continuation the same authorized objective,
   a recovery-only instruction, and the source model and reasoning-effort values
   explicitly through `send_message_to_thread`'s `model` and `thinking` fields.
   It MUST use the authenticated source parent when one exists; otherwise, it
   MUST use the current authorized owner. This message carries existing
   authority and MUST NOT request new approval. If the tool cannot accept both
   settings or delivery fails or is ambiguous, the owner MUST preserve the
   result and stop before archival.

4. The continuation's first task action MUST be native `get_goal`. It MUST NOT
   assume the fork starts with a particular goal state:
   - An exact active objective permits recovery preparation only.
   - Exact `null`, a response envelope whose top-level goal is `null`, or exact
     `status=complete` permits creating a fresh goal for the same authorized
     objective, when the host permits it; the continuation MUST read the new
     goal back as active before continuing. It MUST NOT create a narrower
     preparation objective.
   - `blocked`, a different active objective, an error, `unknown`, `missing`,
     malformed output, or any other state MUST be preserved exactly and MUST
     stop the sequence without another fork.

5. The continuation MUST remain in recovery-only mode. The recovery owner MUST
   establish its association with the exact source checkout using an actually
   supported native operation, such as `attach_worktree` when its ownership
   constraints permit. It MUST read back the continuation's native attachment
   identity and compare it with the source. It MUST then run a successful
   read-only process at the exact intended CWD and compare the repository root,
   Git directories, HEAD, branch, index, tracked and untracked changes, ignored
   data, and worktree inventory with the captured source state. A path string,
   tool discovery, attachment readback, or successful pre-archive process alone
   is insufficient to prove checkout survival.

   Before archival, the owner MUST also establish an explicit documented host
   lifetime guarantee or a supported native operation whose contract covers
   preservation of this exact source checkout through archival of this source
   task. A successful `attach_worktree` call qualifies only when the host's
   documented behavior covers that archive transition for this topology. A
   completed-source diagnostic MUST NOT be generalized to an unfinished blocked
   source. The owner MUST confirm that necessary changes are present or that
   their selected commit/stash objects remain readable from the continuation,
   and that required ignored data is still present at its authorized location.
   It MUST NOT treat a clean index, stash entry, or pre-archive association as
   proof that the selected content or checkout is preserved through archival. If
   the exact association, lifetime-preservation route, process check, or content
   check is unavailable or ambiguous, the owner MUST stop before archival, keep
   the blocked source as owner, preserve its reservation, and report the exact
   unsupported transition and responsible host boundary. It MUST NOT archive
   merely to test whether the checkout survives.

6. When the assignment binds an authenticated parent, the source owner MUST send
   the single `Terminal parent handoff` required for an archive transition by
   [goal-transition reporting](../../orchestration/references/goal-transition-reporting.md).
   The owner MUST include the fork identity, preserved state, any unresolved
   user-input gate, and requested/effective settings evidence. Confirmed
   transport is a receipt, not renewed assignment authorization or parent
   approval. If required delivery fails, the owner MUST keep the source and
   reservation intact and MUST NOT archive.

7. The recovery owner MUST read back that the original source goal remains
   blocked and that the pre-archive association, process, content, and
   lifetime-preservation route checks succeeded for this exact topology. It MUST
   archive the source chat exactly once with
   `set_thread_archived(archived=true)`. It MUST NOT archive the worktree or
   call `update_goal(complete)`. It MUST preserve the exact operation result. If
   it reports failure or is ambiguous, the owner MUST read back source and
   continuation states before naming the owner. If the source task remains
   unarchived, it retains issue-lane ownership even though its goal remains
   blocked, and the continuation MUST stay in recovery-only mode. If the source
   task is archived, the continuation MUST proceed only with the post-archive
   checks below. If owner state is missing, conflicting, or still ambiguous, the
   owner MUST stop issue work in both tasks and report the exact host result
   without retrying the archive.

8. After archival, the recovery owner MUST read back that the source is archived
   and the continuation is the only active owner. It MUST read the
   continuation's exact active objective and settings again, confirm its native
   checkout association and Git worktree inventory, and run a successful process
   at the intended CWD. It MUST compare checkout identity and files with the
   preserved baseline. It MUST also confirm that necessary changes remain
   present or their selected commit/stash objects remain readable, and that
   required ignored data remains present at its authorized location. Only these
   matching post-archive results permit issue work, subject to any unresolved
   user-input gate carried from the source; that gate remains in force until its
   decision owner answers it.

9. If post-archive goal, ownership, association, or process evidence is missing,
   changed, failed, or ambiguous, the continuation MUST remain reserved as the
   sole owner and issue work MUST stop. The owner MUST preserve the exact result
   and recoverable state, report the unsupported step and responsible host
   boundary, and wait for the parent disposition. It MUST NOT fork again,
   restore or replace the checkout to turn a failure into a pass, or invent a
   manual resume command.

## Supported same-task continuation

If a supported native goal-management surface has already resumed the exact
objective in the same task, the owner MUST use `get_goal` and the exact-active
continuation rule in [goal-lifecycle](../SKILL.md). It MUST verify the same
task, owner, checkout, and a successful exact-CWD process before issue work.
This path needs no fork or source archival and does not prove fork/archive
recovery.

## Completed-source diagnostics

A separately authorized, genuinely completed disposable source MAY be used to
inspect checkout association and lifetime. The owner MUST keep that result
limited to the observed attachment topology and process state. A completed-
source result does not establish that an unfinished blocked source can be
recovered, preserve its settings, or continue under its existing assignment. The
owner MUST NOT fabricate a completed goal or blocked state or experiment on live
tasks.

## Native verification boundary

An actual blocked-recovery PASS requires an authorized real blocked source,
evidence that it remained blocked through archival, exactly one continuation
owner, preserved checkout association and identity, the exact authorized active
objective, requested/effective settings kept separate, and a successful
post-archive process at the intended CWD. If no authorized real scenario covers
these conditions, the owner MUST report native recovery as unverified.
Repository checks, instructions, messages, task IDs, and completed-source
diagnostics do not substitute for these readbacks.

For reusable failure interpretation and evidence limits, see
[recovery checkout diagnosis](recovery-checkout-diagnosis.md).
