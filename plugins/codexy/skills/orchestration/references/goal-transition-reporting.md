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

Delegated assignment authorization, exact objective selection, and native goal
startup are defined by [goal-lifecycle](../../goal-lifecycle/SKILL.md). A goal
refresh for the same assignment MUST NOT ask for permission again unless the
next action adds material scope or requires authority not already provided,
including external/destructive authority.

## Runtime Polling Boundary

Polling/monitoring is a runtime claim, not an agent label: a runtime monitor
MUST live outside an execution goal. A Codex heartbeat route is bound by its
heartbeat automation id, target thread, bounded schedule, and last observed
state fingerprint or event identity; it MUST NOT require a persistent
exec/session identifier or same-process resume. A separate process-backed
monitor MAY call an observation only when runtime-issued evidence binds it to a
persistent exec/session identifier, a scheduled next-observation deadline, the
last observed state fingerprint or event identity, and same-process resume.
Repeated model/assistant turn ids, tool-driven re-entry, goal continuation, or
agent invocation without all runtime fields MUST be classified as a continuation
turn unless it is a registered heartbeat automation route, even when each turn
reports that it is polling. Unchanged continuation turns MUST NOT reschedule
themselves or emit another unchanged turn.

An authorized child-local monitor that observes no qualifying event MUST keep
its bounded schedule without emitting status or starting another model turn.
Only a qualifying event or explicit parent/user message may start a new model
turn; This MUST NOT terminate the underlying monitor.

## Ordered Receipts

Native startup and routine readbacks MUST remain task-local under
`goal-lifecycle`, without parent registration receipts.

### Normal completed-phase submission

The child MUST compare its actual active objective with every assigned phase
criterion, required proof, CI, and review, following
[goal-lifecycle](../../goal-lifecycle/SKILL.md). A submitted result, external
wait, or lack of immediate work MUST NOT complete an unfinished objective.

For a satisfied phase, the child MUST prepare exactly one `Idle wait handoff:`.
This is the terminal handoff, not an additional pre-delivery receipt. It MUST
bind `event id`, `issue/pr`, `child task`, `parent task`, `branch`, `worktree`,
`head`, `clean/index`, `last proof`, `current gate`,
`preserved reservation/artifacts`, and `parent next action`. It MUST also carry
`phase objective` matching the active goal, `phase criteria=satisfied`,
`state fingerprint`, nonterminal `producer state`, exact `wake route`,
`ownership=retained`, `issue state=not complete`, `goal transition=complete`,
and `return control=confirmed`.

1. With actual message permission, the child MUST submit that handoff once to
   the authenticated parent. Transport acceptance permits
   `delivery=confirmed; task surface=codex task/thread`. The child MUST NOT call
   `wait_threads` on its parent or await a parent reply or approval for normal
   result submission. Static call evidence uses
   `Child tool call: wait_threads; target=<parent task>`.
2. Without actual message permission, the child MUST prepare the same handoff
   for its host terminal result with
   `delivery=unconfirmed; task surface=host terminal result; message permission=absent`.
   It MUST NOT send an unauthorized message, claim confirmed delivery, retry, or
   manufacture a parent receipt. This fallback preserves the result and
   reservation for parent observation; it does not prove transport acceptance.
3. The child MUST call `update_goal(complete)` only for the satisfied phase and
   preserve the exact native result locally. Static evidence uses
   `Goal tool result: operation=update_goal(complete); status=<native status>`;
   the status-form operation spelling is also supported. A failed, unknown, or
   still-active result MUST retain its actual state and MUST NOT claim idle
   completion. The handoff MUST NOT be resent to obtain a successful goal call.
4. Only after native completion, the child MUST record
   `Idle child state: goal state=complete; plan state=idle`, then return its
   compact terminal result and leave the task idle. That result MUST include the
   exact goal outcome and any unconfirmed delivery limitation. A separate parent
   post-result message MUST NOT be required. Phase completion MUST NOT claim
   issue closure, merge, release, ownership transfer, or parent acceptance.

One event identity MUST NOT authorize a second transition. Duplicate delivery
observations MUST be deduplicated rather than trigger another send or goal call.
Follow-up work stays with the same owner and MUST use the actual lifecycle
readback defined by `goal-lifecycle`, rather than unconditionally creating a
goal. Authentic proof that a completed phase becomes idle without a parent reply
is owned by #1405; validator fixtures MUST NOT claim host behavior.

### Other terminal exits and blocked recovery

For stop, archive, ownership release, or blocked transition outside normal phase
completion, the child MUST send exactly one `Terminal parent handoff:` before
transitioning. It MUST carry the common bound fields above plus
`delivery=confirmed; task surface=codex task/thread`. Static evidence uses
`Terminal child transition: action=stop`, `action=archive`,
`action=ownership release`, or `action=blocked` when no goal operation
represents it. A related terminal stop and ownership release MUST share the same
receipt. A completed-phase handoff authorizes only its idle/stop outcome;
archive or ownership release MUST require a separate confirmed terminal receipt.
If delivery is unavailable, the child MUST emit one unavailable receipt, retain
the lane, and MUST NOT retry or perform the stop/archive/blocked transition.

Blocked-goal recovery retains its ordered receipts. Before
`update_goal(blocked)`, the child MUST send a `Parent goal pre-delivery:` naming
issue/PR, pending operation, parent task, plan step, branch/worktree, HEAD,
dirty/index, evidence, next action, stable transition key, and confirmed
transport acceptance. It MUST NOT call blocked before that acceptance. After the
call, it MUST send `Parent goal post-result:` with the exact tool result,
operation, parent task, matching key, and actual delivery state. The terminal
handoff and intended-transition data MAY share one transport submission; static
records MUST preserve each record's fields and ordering. These blocked-recovery
obligations MUST NOT add pre/post-result messages to normal phase completion.
Runtime delivery mechanics remain owned by #367.

## Blocked Goal User-Decision Gate

Codex MUST reserve `update_goal(blocked)` for missing user input or unanswered
material decisions. Same-objective recovery MUST NOT need new user approval.
Pre-delivery MUST include one typed `Blocked goal user-decision gate:` with ID;
`blocker class=user-decision` or `missing-user-information`;
`decision owner=user`; the exact `user question`; `user response=unanswered`; at
least two distinct `decision branches`; their `material impact`;
`safe default=unavailable`; and `in-scope action=unavailable`. Sentinel, CI,
connector review, parent authorization, dependency integration, resource slots,
alternate evidence routes, and event-idle children are nonterminal and MUST NOT
lead to a blocked call. Repeated turns, fingerprints, elapsed time, token
pressure, difficulty, uncertainty, or incomplete work MUST NOT authorize a
blocked goal.

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

A child with an immediately executable in-scope obligation that is waiting on an
external event MUST use `Nonterminal wait handoff:` with a stable state
fingerprint, nonterminal producer, wake route, `ownership=retained`,
`goal state=active`, `plan state=active`, `goal transition=none`, and
`return control=confirmed`; it MUST NOT call `update_goal(complete)` or
`update_goal(blocked)` merely for that wait.

When every assigned finite-phase criterion and proof is satisfied, the child
MUST use the normal completed-phase submission above. An external wait alone
MUST NOT satisfy the phase. The idle handoff MUST preserve the issue owner,
branch/worktree, artifacts, remaining external gates, and explicit parent wake
route without polling, interrupting, replacing, or approving a live producer.

## Static Recovery Shapes

Static validator fixtures MUST cover representative handoff shapes: #360 and
#276 blocked notices, #311 and #365 usage-limited notices, and #350 task-CWD
versus canonical reserved worktree mismatch. These are evidence-contract
fixtures only; they MUST NOT claim runtime allocator, archive, replacement, or
freeze behavior owned by #373.

When a fixture shows a task CWD that differs from the canonical reserved
worktree, it MUST report the mismatch before any goal continuation evidence.

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
