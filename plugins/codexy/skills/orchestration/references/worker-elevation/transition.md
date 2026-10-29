# Worker elevation transition

## Purpose and boundary

This procedure transfers one active Luna/Max issue lane to a same-issue
Terra/Medium Worker after the Orchestrator records **Elevation may proceed**
under [the elevation decision](../worker-elevation.md) and confirms the
[source checkpoint](checkpoint.md). The existing source remains the sole
implementation owner until the target's read-only setup, identity, goal, model
assignment, parent route, and same-checkout state are confirmed and the source
is safely retired.

This procedure describes request routes and their evidence limits. A request
MUST NOT be reported as a successful native transition. It does not imply a
packaged parent-loop entrypoint: before each step, the Orchestrator MUST check
that the required operation and readback are exposed in the current task. If a
required operation or result is unavailable, the current owner MUST remain in
place and the parent MUST report the exact unsupported step. The target's first
turn MUST be explicitly limited to read-only setup, and the no-write instruction
MUST remain in force until ownership transfers. This is cooperative control, not
proof that future continuation is disabled. If a target write occurs or the
parent cannot confirm that setup stayed read-only, the source MUST remain owner.

## Preconditions

Before forking, the Orchestrator MUST re-read the issue and parent instructions,
its own exact active overall goal, the source's exact active goal, the current
eligibility decision, separate transition authorization, and source checkpoint.
The source MUST remain the authenticated issue owner under a cooperative
no-write instruction that stays in force across continuation turns. Every
identified file-writing producer or tool call MUST have an actual completion or
termination result. A quiet status, ACK, or unchanged Git state alone does not
prove quiescence.

The parent MUST verify and record the source's project, CWD, branch, HEAD,
staged and unstaged changes, required untracked and ignored files, lock
evidence, review obligations, and unresolved failure history. The parent MUST
preserve the existing checkout and MUST NOT require it to be clean or copy
sensitive ignored contents into a public record. A changed checkpoint or new
writer returns preparation to the Orchestrator without a fork.

The parent MUST reserve a target setup slot before forking while retaining the
source reservation. Count both actual occupants against
`min(8, any explicitly lower host or resource limit)`. An absent or ambiguous
thread-list entry, timeout, or pending setup MUST NOT release a reservation. If
capacity is not confirmed, the parent MUST keep the source as owner and leave
elevation pending. The parent MUST retain each reservation until a supported
explicit termination or release is confirmed and the absence of remaining
execution is confirmed. Archival or a missing thread-list entry alone is
insufficient; keep the reservation while either fact is unknown.

## Ordered transition

### 1. Prepare one inert target

The Orchestrator MUST use the supported same-directory fork route with the exact
source task ID:

```text
fork_thread({
  threadId: "<source thread id>",
  environment: { type: "same-directory" }
})
```

This callable has no turn or cursor-boundary argument, and its contract says a
fork may include an interrupted active turn. The Orchestrator MUST wait until
the source's current turn has completed under the no-write instruction, then
recheck the checkpoint immediately before calling it. The fork result alone does
not prove which history boundary was captured.

The Orchestrator MUST use the returned final `threadId` and MUST NOT treat a
pending `clientThreadId` as a target. It MUST NOT request a worktree fork,
create a replacement task, or copy the diff. The source remains the owner and
MUST perform no implementation writes. The Orchestrator MUST read back the
target ID and confirm its lineage, host, project, CWD, same checkout, branch,
HEAD, index, and required files against the frozen source checkpoint. A fork
error, timeout, or ambiguous result MUST be reconciled from actual source and
target state before any further action; MUST NOT create a second fork because a
target is absent from a list.

### 2. Request Terra/Medium and start read-only setup

Before the target's first inference, select `gpt-6.1-terra` / `medium` through
the Codex desktop model and reasoning control beneath the composer when that
pair is available. The official control documents model and effort selection; a
visible selection is only a request, not proof of authenticated assignment. If
Terra/Medium is unavailable, stop with the source still owning the issue.

The callable `send_message_to_thread` also accepts `model` and `thinking`, but
it starts a turn. The first target turn MAY be used only for the read-only setup
below, with the explicit pair and a prompt that prohibits implementation and
checkout writes. MUST NOT send Terra/Medium to a target that remains
authenticated as Luna, override the live source, rely on the sender's pair or a
host default, or substitute raw App Server or CLI task creation. Tool acceptance
establishes transport only; record requested, accepted, authenticated, effective
settings, and actual execution separately.

### 3. Complete target read-only setup and parent-route checks

The target's first action MUST be its own `get_goal` readback. If the exact
assigned phase objective is active, it MUST continue that objective without
replacement. For exact null, a top-level `goal: null`, or exact
`status=complete`, it MUST create the same authorized finite phase objective and
immediately read back that exact objective as active. A blocked, different
unfinished, unknown, malformed, or error result MUST stop the target before any
other work. MUST NOT assume that a fork copies the source goal or permissions.

The target MAY inspect only its native task identity, project, CWD, checkout,
branch, HEAD, index, and required files. It MUST confirm these match the source
checkpoint and MUST NOT edit, stage, commit, create a branch, run implementation
checks, or start another writer. Its host-authenticated model assignment MUST
match the requested Terra/Medium pair before retirement and implementation.
Missing, unauthenticated, or mismatched assignment MUST stop the transition.
Record observed effective settings separately when the host exposes them; if
unavailable, record them as unavailable and do not infer their value. An
explicitly observed wrong effective pair MUST stop before retirement and
implementation, but unavailable effective telemetry alone MUST NOT invalidate an
otherwise confirmed authenticated assignment.

The target MUST inspect its own `list_artifacts({})` result to confirm the
inherited managed-checkout attachment. If the exact existing workspace from the
source checkpoint needs to be reattached, it MAY call
`attach_worktree({path: "<exact inherited workspace directory>"})`. This
reattaches the existing checkout without resetting files or running setup
scripts, and may update the target task's workspace association. The target MUST
use only the exact path established by the inherited attachment or source
checkpoint; it MUST NOT create a worktree, guess a path, or copy files. After
reattachment, it MUST read back its workspace directory, attachment, branch,
HEAD, index, and required files again. The attachment result alone does not
prove checkout identity or survival after source archival.

The target MUST verify the bidirectional route to the exact original
Orchestrator ID supplied by the authenticated assignment. The target MUST send
one authenticated report to that exact parent using
`send_message_to_thread({threadId, hostId, model: "gpt-6.1-sol", thinking:
"low", prompt})`;
transport acceptance alone is not a parent callback. The Orchestrator MUST
observe the report on its task and MUST send a follow-up to the exact target ID
using the authenticated target pair. The target MUST observe that follow-up.
This confirms message routing, not approval. The Orchestrator MUST also provide
its fresh native readback of the exact active overall goal; the target cannot
query another task's goal directly.

### 4. Recheck before source retirement

Before transfer, the Orchestrator MUST recheck the target's exact identity,
authenticated pair, active objective, parent route, and same-checkout state. It
MUST also recheck the source's no-write instruction, actual writer completion,
original ownership, active goal, checkpoint, and current parent instructions.
Any new writer, changed file state, missing or mismatched binding, or ambiguous
readback MUST stop advancement. The Orchestrator MUST keep both actual
reservations and preserve both tasks and the checkout for parent disposition.

### 5. Deliver the existing terminal handoff and retire the source

The source MUST send exactly one `Terminal parent handoff:` under the existing
[goal-transition reporting contract](../goal-transition-reporting.md), using its
authenticated parent ID, a fresh event identity, and the Worker-to- Orchestrator
pair. The source MUST preserve the contract's exact common bound fields and
`delivery=confirmed; task surface=codex task/thread`. It MUST add the issue/PR,
source and target IDs, objective, branch/worktree/HEAD and dirty-index state,
source writer results, target goal/environment/model/parent readbacks, review
and failure history, remaining checks, and requested ownership transfer. The
source MUST NOT reuse the event identity for a second transition. Transport
acceptance is not parent approval or proof of a callback. Once the authenticated
handoff is accepted, an unobserved callback MUST remain a separate unknown and
MUST NOT add a parent-reply or callback-observation gate to source retirement.

Before issuing the archive request, the Orchestrator MUST establish that it is
source-only and will preserve the target and original checkout, including the
effects on any descendants. If source-only behavior, descendant effects, or
checkout preservation cannot be established, the Orchestrator MUST NOT archive,
transfer ownership, or authorize implementation; it MUST retain both tasks,
reservations, and the checkout for parent disposition.

After that accepted handoff and precondition, the Orchestrator MAY issue the
exposed request
`mcp__codex_app__set_thread_archived({threadId: "<exact source task ID>", archived: true})`.
Whether it archives descendants remains unobserved. The Orchestrator MUST read
back the source archive result and verify that the target and original checkout
remain available. No standalone stop or release call is established by this
contract. Archival does not prove that the active source goal terminated, that
no execution remains, or that automatic continuation is disabled. If the
readback is ambiguous, the Orchestrator MUST NOT transfer ownership; it MUST
keep the source owner and leave the transition pending. It MUST NOT change the
source goal to `complete` or `blocked` to retire it. After transfer, any resumed
source turn MUST remain read-only and MUST NOT reclaim ownership.

The Orchestrator MUST retain both reservations until it has confirmed a
supported explicit termination or release and the absence of remaining
execution, including relevant descendants. Archival success or a missing
thread-list entry alone MUST NOT release capacity. Because no standalone stop or
release operation is established here, keep each reservation and route its
disposition to the parent while either condition remains unverified.

### 6. Replace the owner and authorize target implementation

Only after source retirement and the target's identity and authenticated pair
are confirmed may the Orchestrator update its existing owner record to the
target. The Orchestrator MUST preserve the issue/PR, objective, branch,
checkout, staged and unstaged work, required artifacts, reviewer, unresolved
findings, failure history, and reservations. The target MUST NOT implement until
the parent has made the single-owner transfer after all required readbacks.

The target MAY implement only after a final readback confirms its exact active
goal, assignment, permissions, pair, same checkout, and ownership. The source
MUST remain read-only. The parent determines resumed work from the target's
actual next tool call, diff, or result.

## Failure handling

| Failure boundary                                                                                                            | Required handling                                                                                                                                                                                                      |
| --------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Source state, permissions, checkpoint, parent instructions, or capacity changed                                             | MUST NOT fork. Keep the source owner and reconcile with the Orchestrator.                                                                                                                                              |
| Fork errors, times out, or returns an ambiguous identity                                                                    | MUST inspect actual source and target records. MUST NOT fork again based on an absent list entry. MUST retain all reservations.                                                                                        |
| Target model request is unavailable or rejected, or its authenticated assignment is missing, unauthenticated, or mismatched | MUST NOT retire the source or implement. Preserve both IDs and reservations; report requested, accepted, authenticated, effective, and actual-execution evidence separately.                                           |
| Effective settings are not exposed                                                                                          | Record them as unavailable and do not infer their value. Continue only when the authenticated target assignment is confirmed as Terra/Medium.                                                                          |
| An explicitly observed effective pair is wrong                                                                              | MUST NOT retire the source or implement. Preserve both IDs and reservations and report the observed mismatch.                                                                                                          |
| Target goal is blocked, different, malformed, or unknown                                                                    | MUST stop before further setup or implementation. MUST NOT fabricate or replace a goal.                                                                                                                                |
| Parent route or parent active-goal evidence is missing                                                                      | MUST NOT retire the source or implement. Preserve the target and checkout; route the exact gap to the parent.                                                                                                          |
| Source handoff is not accepted or authenticated                                                                             | MUST NOT retire the source or transfer ownership. If accepted, an unknown callback remains separate and MUST NOT require a parent reply or another callback observation.                                               |
| Source-only archival, descendant effects, or checkout preservation cannot be established before the request                 | MUST NOT archive, transfer ownership, or authorize implementation. Keep both tasks, reservations, and checkout for parent disposition.                                                                                 |
| Source retirement fails or its preservation effect is ambiguous after the request                                           | Neither task may implement or transfer ownership. MUST keep identities, checkout, and reservations for parent disposition.                                                                                             |
| Reservation release is requested                                                                                            | MUST retain the reservation until supported explicit termination or release and absence of remaining execution, including relevant descendants, are confirmed. Archival or a missing list entry alone is insufficient. |
| Target assignment, environment, or goal fails after retirement                                                              | MUST keep the target and unfinished-issue reservation. MUST NOT revive the source or create another fork.                                                                                                              |
| First continuation acceptance is ambiguous                                                                                  | MUST inspect events on the known target. MUST NOT resend or fork again before reconciliation.                                                                                                                          |

Retry requires a separate parent instruction after reconciliation and a
supported recovery route. MUST NOT build an automatic retry ladder or run an
unassigned native fork/retirement scenario. Unsupported or unobserved behavior
keeps the transition pending; fixtures and local checks cannot prove native
success.
