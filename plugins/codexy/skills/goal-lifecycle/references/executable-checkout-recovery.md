# Executable-checkout recovery

This reference is part of [goal-lifecycle](../SKILL.md). It defines same-scope
recovery of an unfinished blocked assignment. It MUST NOT authorize a new task,
clear a goal, or permit issue work before recovery acceptance.

## Authority and safety boundary

When the original assignment still authorizes the same objective and scope, that
authority carries through recovery. A blocked state alone MUST NOT require
another user confirmation. An explicit user stop, revoked authority, changed
scope, missing original assignment, or actual host permission denial MUST stop
the sequence; preserve the source owner and report the exact reason. Host goal
and tool contracts remain authoritative.

An unfinished blocked goal MUST NOT be marked complete or cleared to obtain
recovery. Recovery preparation MUST be limited to preserving necessary owned
work, preparing or reusing one continuation at a time, checking the intended
checkout, and archiving the source chat. Issue implementation and ordinary issue
verification MUST remain stopped until the continuation is the sole owner, its
exact authorized objective is active, and a successful process runs at the
preserved checkout.

Recovery does not answer or clear an existing user-input gate. If the source has
an unanswered material user question, the owner MUST preserve that exact
question and its decision owner under the
[blocked-goal user-decision gate](../../orchestration/references/goal-transition-reporting.md#blocked-goal-user-decision-gate).
A fork, active-goal readback, parent report, or archive MUST NOT count as the
answer. Recovery preparation MAY proceed under the existing assignment, but
issue work that depends on the missing input MUST remain stopped until the
decision owner supplies it.

## Source preflight and owned work

Before preparing a continuation, the owner MUST capture:

- The exact blocked `get_goal` result, source task and authenticated parent,
  original authorized objective and scope, and current ownership.
- The exact recovery blocker and any unresolved user-input gate. The owner MUST
  carry them into the continuation and MUST NOT treat recovery as answering the
  gate.
- Requested model and reasoning effort from the source assignment or creation
  record, separately from effective settings that the host exposes. For each
  continuation field, the owner MUST use the exposed effective value or its
  exact requested value. If a value is unavailable from both sources, the owner
  MUST stop before forking and report that exact gap.
- The exact assigned CWD and a lightweight successful read-only process there:
  canonical repository root, absolute Git directory and common directory, HEAD,
  branch or detached state, and `git status --short --branch`.
- Necessary owned changes and required ignored data, without disclosing
  secrets. Preserve only in-scope work in its authorized location; resolve
  unexpected data or unclear ownership before continuing.
- Confirmation that no source or continuation process can write implementation
  changes during recovery preparation.

The owner MUST reuse a valid process and checkout read already obtained for the
same continuation while its state remains unchanged. It MUST NOT require
`list_artifacts`, `attach_worktree`, an exposed attachment identity, or a
separate lifetime guarantee before archiving. `list_artifacts({})` reports
attachments for the calling chat and may return no worktree or only pull
requests; that result MUST NOT be used to infer another chat's checkout state.

A scoped commit or stash MAY preserve selected owned changes. The owner MUST
preserve required ignored data separately. After any preservation write, it
MUST recapture the exact-CWD Git state and use it as the continuation's
baseline. It MUST NOT stage or disclose credentials. The owner MUST NOT treat a
saved snapshot or unchanged HEAD alone as proof that a process can run at the
intended checkout.

## Ordered blocked-goal procedure

The source MUST remain the issue owner until its chat archival is confirmed. The
recovery owner MUST coordinate the steps below; a continuation MUST perform
only recovery preparation until the source is archived and post-archive
acceptance succeeds. An authenticated parent MAY coordinate these actions
under the existing assignment, but a message or child report MUST NOT replace
native goal, fork, archive, or process readbacks.

1. The recovery owner MUST confirm that the original assignment still
   authorizes the same objective and scope. It MUST NOT request confirmation
   solely because the goal is blocked. If authority or scope is absent, stopped,
   revoked, or changed, it MUST preserve the source and stop.

2. The recovery owner MUST identify the source owner and any already prepared
   continuation. It MUST reuse an authenticated same-directory fork when its
   task, objective, and checkout match this recovery. Otherwise, it MUST create
   one same-directory fork with the supported `fork_thread` operation. It MUST
   NOT create a competing branch or worktree as the default recovery route.

3. The recovery owner MUST preserve only necessary owned work and source
   settings. It MUST send the continuation the same authorized objective, scope,
   and a recovery-only instruction. It MUST use the source's requested `model`
   and `thinking` values explicitly when the host message surface requires
   them; requested and host-exposed effective values MUST remain separate. If
   the host cannot carry the required values, delivery fails, or the result is
   ambiguous, the owner MUST preserve the exact result and stop before source
   archival.

4. The continuation's first task action MUST be native `get_goal`. It MUST NOT
   assume the fork starts with a particular goal state:
   - An exact active objective permits recovery preparation only.
   - Exact `null`, a response envelope whose top-level goal is `null`, or exact
     `status=complete` permits creating a fresh goal for the same authorized
     objective when the host permits it; the continuation MUST read the new
     goal back as active before continuing. It MUST NOT create a narrower
     preparation objective.
   - A `blocked` result for the same objective MUST stay in recovery-only mode
     and proceed through checkout verification and source-chat archival.
   - A different active objective, an error, `unknown`, `missing`, malformed
     output, or any other state MUST be preserved exactly and MUST stop the
     sequence.

5. The recovery owner MUST verify the continuation's actual checkout using its
   fork result, exact CWD, canonical repository root, absolute Git directory and
   common directory, HEAD, branch state, and a successful lightweight
   read-only process. It MUST compare these with the source baseline and confirm
   necessary owned changes and required ignored data remain available. A
   matching CWD and process result establish the checkout at this observation
   point; an artifact listing or separate host lifetime guarantee is not
   required.

   If the CWD process fails or the Git checkout does not match, the recovery
   owner MUST use a supported operation to repair that observed failure before
   archival. A missing, unsupported, or failed attachment listing alone is not
   a checkout failure. If the actual checkout failure cannot be repaired, the
   owner MUST preserve the source as owner and report the exact operation and
   host result.

6. When the assignment binds an authenticated parent, the owner MUST send the single
   `Terminal parent handoff` required for an archive transition by
   [goal-transition reporting](../../orchestration/references/goal-transition-reporting.md).
   It MUST include the continuation identity, preserved state, unresolved
   user-input gate, and requested/effective settings evidence. Confirmed
   transport is a receipt, not renewed assignment authorization or parent
   approval. If required delivery fails, it MUST keep the source and reservation
   intact.

7. The recovery owner MUST read back that the original source goal remains
   blocked and that checkout verification succeeded for this continuation. It
   MUST archive the source **chat** exactly once with
   `set_thread_archived(archived=true)`. It MUST NOT archive the worktree or
   call `update_goal(complete)`. It MUST preserve the exact operation result. If
   archival fails or is ambiguous, it MUST read back source and continuation
   states before naming the owner. If the source remains unarchived, it retains
   issue ownership and the continuation MUST stay in recovery-only mode. If the
   source is archived, the continuation MUST proceed to post-archive checks. If
   one owner cannot be confirmed, the recovery owner MUST stop issue work and
   report the exact host result.

8. After archival, the recovery owner MUST read back that the source is archived
   and the continuation is the sole owner. It MUST read the source goal state
   through archive when the host exposes it; if that readback is unavailable, it
   MUST report that the source's blocked state through archival is unverified.
   It MUST read the continuation's goal and settings, then run a successful
   lightweight process at the intended CWD and compare Git checkout identity
   and necessary files with the preserved baseline. It MUST NOT require an
   attachment artifact or lifetime guarantee when these checks succeed.
   - The exact authorized active objective plus matching checkout and process
     permit issue work, subject to any unresolved user-input gate.
   - If the same goal is still blocked, the recovery owner MUST keep issue work
     stopped and continue this procedure from that sole current owner, reusing a
     suitable continuation if one already exists.
   - A permitted fresh-goal result MUST be read back as active before issue
     work. The recovery owner MUST preserve and report a different objective,
     an error, or an ambiguous owner state, then stop.

9. If the post-archive process or checkout comparison shows an actual failure,
   the recovery owner MUST repair that failure with a supported operation while
   preserving the sole owner and its work. If it cannot be repaired, the owner
   MUST keep issue work stopped and report the exact host operation, result, and
   recoverable state. An absent or unsupported artifact result MUST NOT
   invalidate a successful process.

## Supported same-task continuation

If a supported native goal-management surface has already resumed the exact
objective in the same task, the owner MUST use `get_goal` and the exact-active
continuation rule in [goal-lifecycle](../SKILL.md). It MUST verify the same
task, owner, checkout, and a successful exact-CWD process before issue work.
This path needs no fork or source archival and does not prove fork/archive
recovery.

## Completed-source diagnostics

A separately authorized, genuinely completed disposable source MAY be used to
inspect checkout behavior. The owner MUST keep that result limited to the
topology and process state actually observed. A completed-source result does
not establish that an unfinished blocked source can be recovered or continue
under its existing assignment. The owner MUST NOT fabricate a completed goal
or blocked state or experiment on live tasks.

## Native verification boundary

An actual fork-recovery PASS requires an authorized real blocked source, proof
that its goal remained blocked through chat archival, exactly one continuation
owner, the same authorized objective and checkout, requested/effective settings
kept separate, and a successful post-archive process at the intended CWD. It
does not require a `list_artifacts` result, attachment identity, or separate
lifetime guarantee when the actual process and Git identity checks pass. If no
authorized real scenario covers these conditions, the owner MUST report native
recovery as unverified. Repository checks, instructions, messages, task IDs,
and completed-source diagnostics MUST NOT substitute for these readbacks.

An actual same-task recovery PASS requires a real supported native resumption
and the same-task acceptance readbacks above. It MUST NOT imply fork/archive
recovery passed.

For reusable failure interpretation and evidence limits, see
[recovery checkout diagnosis](recovery-checkout-diagnosis.md).
