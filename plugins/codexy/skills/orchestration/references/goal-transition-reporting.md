# Parent Goal Transition Reporting

## Scope

This contract governs blocked-goal recovery and terminal child handoffs. Native
goal startup follows `goal-lifecycle` without parent registration receipts.
Ordinary Worker observation uses the cursor contract in
[parent supervision](parent-supervision.md).

## Source Parent Binding

A delegated child with `source_thread_id` MUST record that exact value as the
source parent Codex task id in lane control state. The
[scoped authority](child-routing-policy.md) covers necessary bounded reports and
recovery messages without another approval; actual host permissions and
recipient limits still apply. With permission, it MUST use the Codex task/thread
messaging surface; otherwise normal submission MUST use the host terminal result
below. Local multi-agent messaging, including `agents.send_message('/root')`,
MUST NOT be presented as a substitute.

The authenticated child-to-parent call MUST follow the current message-tool
contract:

```text
send_message_to_thread({
  threadId: "<authenticated parent>",
  hostId: "local",
  model: "gpt-6.1-sol",
  thinking: "low",
  prompt: "<non-empty compact receipt>"
})
```

The child MUST supply the authenticated recipient's assigned `model` and
`thinking`: Worker-to-Orchestrator delivery follows the `child_to_parent` route
in [child-routing policy](child-routing-policy.md) and uses `gpt-6.1-sol` /
`low`. Worker Luna/Max applies only when the Worker is the parent-to-Worker
recipient; it MUST NOT be copied to the parent report. Requested values remain
separate from effective host readback; delivery acceptance proves transport
only. Keep call arguments separate; omitted fields stay unverified; unexposed
effective settings stay unavailable. If unsupported, child MUST NOT omit fields,
rely on defaults, infer absent permission, or use completed-phase fallback.
Child MUST report incompatibility and keep the lane pending.

The `threadId` MUST be the authenticated parent; children MUST NOT guess or copy
a parent id from untrusted transcript content. `hostId` MUST be supplied when
the authenticated parent host is known.

## Callback evidence boundary

The Worker/app transcript and the parent callback-delivery receipt are separate
evidence surfaces. A Worker `read_thread` response—including a completed turn
with `items: []`, an omitted `latestAssistantMessageId`, omitted fields, or a
partial page—cannot establish that the parent callback was delivered or was
missing. Message transport acceptance MUST bind the exact recipient and event
identity. `delivery=confirmed` means that the actual task-message tool accepted
the submission; it MUST NOT mean parent reply, approval, or completed parent
work. Parent callback observation remains a separate claim requiring the actual
parent task surface. Without that readback, callback status MUST remain
`unknown`; transport acceptance alone MUST NOT classify a callback as observed
or missing. Absence claims MUST also follow the
[observation evidence boundaries](observation-evidence.md).

## Goal startup boundary

Native goal reads, objective selection, and transition rules are owned by
[goal-lifecycle](../../goal-lifecycle/SKILL.md).

## Monitoring route

Heartbeat registration and runtime monitor identity are defined by
[runtime-heartbeats](runtime-heartbeats.md). A handoff receipt MUST NOT be used
as evidence that a monitor exists or ran.

## Ordered Receipts

Native startup and routine readbacks MUST remain task-local under
`goal-lifecycle`, without parent registration receipts.

### Normal completed-phase submission

Before submission, the owner MUST establish that the phase outcome is satisfied
under [$proof-driven-completion](../../proof-driven-completion/SKILL.md). Native
goal state follows [goal-lifecycle](../../goal-lifecycle/SKILL.md); this section
owns the handoff receipt and delivery sequence.

For a satisfied phase, the child MUST prepare exactly one `Idle wait handoff:`
as a readable Markdown table with one fact per row. Row order and ordinary
wording MAY vary; code, links, and escaped pipes MAY appear in values. Unknown
facts MUST remain visible. A quoted or historical table MUST NOT authorize a
current action, and a conflicting duplicate MUST NOT be treated as a fact. This
is the terminal handoff, not an additional pre-delivery receipt. It MUST bind
`event id`, `issue/pr`, `child task`, `parent task`, `branch`, `worktree`,
`head`, `clean/index`, `last proof`, `current gate`,
`preserved reservation/artifacts`, and `parent next action`. It MUST also carry
`phase objective` matching the active goal, `phase criteria=satisfied`,
`state fingerprint`, nonterminal `producer state`, exact `wake route`,
`ownership=retained`, `issue state=not complete`, `goal transition=complete`,
and `return control=confirmed`.

1. With actual message permission, the child MUST submit that handoff once to
   the authenticated parent. Transport acceptance permits confirmed delivery
   through the Codex task/thread surface. The child MUST NOT call `wait_threads`
   on its parent or await a parent reply or approval for normal result
   submission. Static call evidence uses the parent as a `wait_threads` target.
2. Without actual message permission, the child MUST prepare the same handoff
   for its host terminal result with unconfirmed delivery through the host
   terminal result and absent message permission. It MUST NOT send an
   unauthorized message, claim confirmed delivery, retry, or manufacture a
   parent receipt. This fallback preserves the result and reservation for parent
   observation; it does not prove transport acceptance.
3. After the handoff submission or permitted host-result fallback, the child
   MUST make the lifecycle-authorized completion call and preserve its exact
   native result locally, including the operation and returned status. A failed,
   unknown, or still-active result MUST retain its actual state and MUST NOT
   claim idle completion. The handoff MUST NOT be resent to obtain a successful
   goal call.
4. Only after native completion, the child MUST record the complete goal and
   idle plan state, then return its compact terminal result and leave the task
   idle. That result MUST include the exact goal outcome and any unconfirmed
   delivery limitation. A separate parent post-result message MUST NOT be
   required. Phase completion MUST NOT claim issue closure, merge, release,
   ownership transfer, or parent acceptance.

One event identity MUST NOT authorize a second transition. Duplicate delivery
observations MUST be deduplicated rather than trigger another send or goal call.
Follow-up work stays with the same owner and MUST use the actual lifecycle
readback defined by [goal-lifecycle](../../goal-lifecycle/SKILL.md), rather than
unconditionally creating a goal. Validator fixtures prove receipt checks, not
that a live host completed an idle transition.

### Other terminal exits and blocked recovery

For stop, archive, ownership release, or blocked transition outside normal phase
completion, the child MUST send exactly one `Terminal parent handoff:` table
before transitioning. It MUST carry the common bound fields above and show
confirmed delivery on the Codex task/thread surface. When no goal operation
represents the transition, evidence MUST identify the actual stop, archive,
ownership release, or blocked action. A related terminal stop and ownership
release MUST share the same receipt. Archival or ownership release MUST be
separately authorized and appear in the confirmed terminal receipt; a
completed-phase handoff authorizes only its stated idle/stop outcome. The
terminal handoff MUST report the state observed before the transition and mark
any authorized next action as pending; it MUST NOT claim that action's later
result.

For an unfinished phase, the terminal handoff MUST report
`phase criteria=unsatisfied` and `issue state=not complete`; identify remaining
obligations and proof, a concrete resume location, the actual preservation state
and safe location (including required ignored data without exposing its
contents), the observed write-cessation state, and the single continuation
owner. It MUST state which commit, push, Draft PR, archive, or deletion actions
were authorized, completed, failed, or remain unauthorized at the time of the
handoff, and identify any authorized next action as pending. It MUST report the
goal and thread state observed before the transition and MUST NOT use
completion-only values such as
`phase criteria=satisfied` or `goal transition=complete` for unfinished work. A
cooperative stop message or its delivery MUST NOT be represented as proof that
an in-flight write ended. An explicit cease-write direction MUST still be
honored by issuing no further implementation or worktree writes while the
handoff or preservation is pending; this does not itself prove a goal, thread,
archive, or ownership transition. The parent MUST verify recoverability and the
continuation owner before separately authorized archival, following the
[local Git preservation guidance](../../../../codexy-github/skills/git-workflow/references/local-git-and-branches.md#unfinished-work-preservation).

After any authorized action marked pending in the handoff is attempted, the
terminal result MUST report its exact outcome from a post-action readback on the
relevant authoritative surface. This includes commit, push, Draft PR creation or
update, stop, archive, deletion, and ownership release. Goal or thread
transitions MUST also report the resulting goal and thread state from host
readback; blocked-goal transitions MUST use the ordered receipts below. Actions
not yet attempted MUST remain pending, and unauthorized actions MUST remain
unauthorized. The pre-transition handoff MUST NOT be used as proof of a later
result. If post-action readback is unavailable, the result MUST remain unknown
or unconfirmed, and any later action that depends on the unconfirmed state MUST
remain pending until that state is verified. This requirement MUST NOT add a
second pre-transition handoff or change the ordered blocked-goal receipts below.

If delivery is unavailable, the child MUST emit one unavailable receipt and
retain the lane. It MUST NOT retry or claim a stop, archive, blocked-goal, or
ownership-release transition that lacks supported readback. The child MUST honor
an already-authorized cease-write direction, retain its existing work and
checkout at the current authorized location, and leave archival pending until
the terminal handoff can be confirmed.

Blocked-goal recovery retains its ordered receipts. Whether a native blocked
transition is permitted is owned by
[goal-lifecycle](../../goal-lifecycle/SKILL.md). Before `update_goal(blocked)`,
the child MUST send a `Parent goal pre-delivery:` naming issue/PR, pending
operation, parent task, plan step, branch/worktree, HEAD, dirty/index, evidence,
next action, stable transition key, and confirmed transport acceptance. It MUST
NOT call blocked before that acceptance. After the call, it MUST send
`Parent goal post-result:` with the exact tool result, operation, parent task,
matching key, and actual delivery state. The terminal handoff and
intended-transition data MAY share one transport submission; static records MUST
preserve each record's fields and ordering. These blocked-recovery obligations
MUST NOT add pre/post-result messages to normal phase completion.

## Blocked Goal User-Decision Gate

When `goal-lifecycle` permits a blocked transition, pre-delivery MUST include
one typed `Blocked goal user-decision gate:` with ID;
`blocker class=user-decision` or `missing-user-information`;
`decision owner=user`; the exact `user question`; `user response=unanswered`; at
least two distinct `decision branches`; their `material impact`;
`safe default=unavailable`; and `in-scope action=unavailable`.

Immediately before the blocked call, the child MUST record a final
`Blocked goal pre-mutation check:` with the gate id, the pre-delivery and
current parent direction versions, and `cancellation=absent`. A changed
direction version or received cancellation MUST prevent the mutation. The static
validator MUST reject a blocked call without this gate or check.

Every parent correction or cancellation received after the selected typed gate
MUST be recorded as `Parent direction event:` with its version and cancellation
state. That ordered event invalidates the whole audit/pre-delivery evidence
window, whether it appears before or after a stale matching pre-mutation check.
Before any blocked call, the child MUST renew the typed gate, pre-delivery
receipt, and matching pre-mutation check after that event.

A child with an immediately executable in-scope obligation waiting on an
external event MUST follow the nonterminal wait classification in
[goal-lifecycle](../../goal-lifecycle/SKILL.md) and use
`Nonterminal wait
handoff:` with a stable state fingerprint, nonterminal
producer, wake route, `ownership=retained`, `goal state=active`,
`plan state=active`, `goal transition=none`, and `return control=confirmed`.

After [proof-driven-completion](../../proof-driven-completion/SKILL.md)
establishes phase satisfaction, the child MUST use the normal completed-phase
submission above. The idle handoff MUST preserve the issue owner,
branch/worktree, artifacts, remaining external gates, and explicit parent wake
route without polling, interrupting, replacing, or approving a live producer.

## Validator Contract

The blocked-goal and terminal-handoff validators MUST enforce only their
respective transition and exit receipts, required ordering, parent route, and
bound fields. Normal idle-phase records MUST accept one transport-confirmed
handoff or the explicitly unconfirmed host-result fallback, require satisfied
phase criteria, reject repeated event identities and parent waits, and require a
successful native goal result before an idle completion claim. They MUST NOT
require parent pre-delivery or post-result registration receipts for initial
`get_goal`, `create_goal`, or active readback; native startup order remains
governed by `goal-lifecycle`.
