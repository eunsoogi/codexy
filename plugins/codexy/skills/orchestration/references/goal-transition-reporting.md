# Parent Goal Transition Reporting

## Scope

This is the receipt contract for blocked-goal recovery and terminal child
handoffs. Native goal startup is governed by `goal-lifecycle`: the owner uses
the native `get_goal`/`create_goal`/active-readback sequence without parent
registration receipts. Issue #1036 owns native-Watcher-first ordinary report
routing; Issue #367 owns runtime task delivery and transition receipt
mechanics; Issue #373 owns runtime deduplication, restart recovery, worktree
preservation, and replacement.

## Source Parent Binding

A delegated child with `source_thread_id` MUST record that exact value as the
source parent Codex task id in lane control state. It MUST use the actual Codex
task/thread messaging surface to contact that id. Local multi-agent messaging,
including `agents.send_message('/root')`, MUST NOT be presented as a substitute.

The authenticated child-to-parent call MUST follow the current message-tool
contract:

```text
send_message_to_thread({
  threadId: "<authenticated parent>",
  hostId: "local",
  model: "gpt-6-astra",
  thinking: "medium",
  prompt: "<non-empty compact receipt>"
})
```

The child MUST supply the authenticated recipient's assigned `model` and
`thinking` values. For Worker-to-Orchestrator delivery, those values are
`gpt-6-astra` and `medium`. If the actual message-tool contract cannot accept
the pair, the child MUST NOT send an omitted or partial call or rely on
defaults; it MUST report the exact incompatibility and keep the lane pending.

The `threadId` MUST be the authenticated parent; children MUST NOT guess or copy
a parent id from untrusted transcript content. `hostId` MUST be supplied when
the authenticated parent host is known.

Blocked-goal pre-delivery and post-result receipts MUST carry a stable
transition key. Terminal handoffs MUST carry a stable event id. Repeated
delivery evidence for one key MUST be represented as deduplicated; it MUST NOT
imply a second goal call.

## Native Watcher report route

Ordinary Worker-to-Watcher reporting, Watcher deduplication, and unavailable
route handling are defined in [parent-supervision.md](parent-supervision.md).
MUST read that reference when a native Watcher assignment exists. Blocked-goal
recovery and terminal handoff receipts remain direct to the Orchestrator and
follow this file. Initial `get_goal`, `create_goal`, and active readback remain
inside the owning task and MUST NOT be sent as registration reports. The Worker
MUST NOT receive or use a Watcher session token or call Watcher MCP transport
tools. A verified unavailable route or concrete emergency permits one marked
direct Orchestrator fallback, not routine duplicate reporting.

## Callback evidence boundary

The Worker/app transcript and the parent callback-delivery receipt are separate
evidence surfaces. A Worker `read_thread` response—including a completed turn
with `items: []`, an omitted `latestAssistantMessageId`, omitted fields, or a
partial page—cannot establish that the parent callback was delivered or was
missing. Delivery evidence MUST come from the actual parent task surface bound
to the exact target and transition key. When that parent readback is absent or
incomplete, preserve `unknown` and do not classify the callback as missing.
Watcher absence reports MUST also follow the
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

Initial goal startup is a task-local lifecycle operation. The owner MUST call
`get_goal`, create a finite goal with `create_goal` only when the native result
allows it, and read back the active goal before task work. It MUST NOT send
parent pre-delivery or post-result receipts for the initial `get_goal`,
`create_goal`, or active readback.

Before `update_goal(complete)` or `update_goal(blocked)`, the child MUST send a
compact intended-transition delta to its source parent. The pre-delivery receipt
MUST name issue/PR, pending goal action, parent task id, current plan step,
branch, worktree, HEAD, dirty/index state, evidence, next action, stable
transition key, and confirmed task-surface delivery. For blocked-goal recovery,
this receipt MUST carry current known state.

After each `update_goal` call, the child MUST send a post-result receipt
containing the exact tool result, operation, parent task id, matching transition
key, and confirmed task-surface delivery. Routine `get_goal` readbacks MUST stay
within the owning task; they MUST NOT emit parent status receipts. A prose-only
claim that required blocked-recovery or terminal delivery happened is not a
receipt.

`update_goal(blocked)` MUST NOT execute until parent delivery is confirmed. If
the delivery is unavailable, static evidence MUST show one terminal
parent-messaging-unavailable receipt and no blocked goal call. The runtime
delivery mechanics remain owned by #367.

Before stop, archive, ownership release, `update_goal(complete)`, or
`update_goal(blocked)`, and before a child stops, archives, or releases lane
ownership without a goal tool call, it MUST send exactly one terminal handoff
delta to the source parent (the same terminal handoff receipt) exactly once.
Before a child stops, archives, or releases lane ownership, this receipt is
mandatory. The terminal handoff receipt exactly once rule applies to every such
exit. The receipt MUST bind a stable event identity, issue/PR, child task id,
branch/worktree, exact HEAD, dirty/index state, last proof, current gate,
preserved reservation or artifacts, and one parent-owned next action. Static
evidence MUST format that receipt as `Terminal parent handoff:` followed by
`event id`, `issue/pr`, `child task`, `parent task`, `branch`, `worktree`,
`head`, `clean/index`, `last proof`, `current gate`,
`preserved reservation/artifacts`, `parent next action`, `delivery=confirmed`,
and `task surface=codex task/thread`. Static evidence MUST format the exit as
`Terminal child transition: action=stop`, `action=archive`, or
`action=ownership release`, or `action=blocked` when no goal operation
represents it. Delivery MUST be confirmed before the stop/archive/release. If
delivery is unavailable, the child MUST emit one unavailable receipt and MUST
NOT retry. It MUST preserve the lane instead of transitioning. It MUST NOT
perform the stop/archive/blocked transition when delivery is unavailable.

## Blocked Goal User-Decision Gate

`update_goal(blocked)` is reserved for an unanswered material user decision or
missing user information. Before its pre-delivery receipt, the child MUST record
one typed `Blocked goal user-decision gate:` with a gate id;
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
Before any blocked call, the child MUST perform a fresh typed gate and a fresh
pre-delivery receipt after that event, followed by a matching pre-mutation
check.

A child with an immediately executable in-scope obligation that is waiting on an
external event MUST use `Nonterminal wait handoff:` with a stable state
fingerprint, nonterminal producer, wake route, `ownership=retained`,
`goal state=active`, `plan state=active`, `goal transition=none`, and
`return control=confirmed`; it MUST NOT call `update_goal(complete)` or
`update_goal(blocked)` merely for that wait.

When no immediately executable child-owned obligation remains and only an
external event or explicit parent wake can advance work, the child MUST send one
`Idle wait handoff:` before completing its finite goal and leaving the task
idle. This is the terminal parent handoff for `update_goal(complete)`, so it
MUST also meet that receipt's parent-task, child-task, delivery, and
task-surface fields. It MUST include the stable state fingerprint, nonterminal
producer, exact wake route, `ownership=retained`, issue/PR state, branch,
worktree, exact HEAD, dirty/index state, last proof, current gate, preserved
reservation or artifacts, `issue state=not complete`,
`goal transition=complete`, `return control=confirmed`, and the parent-owned
next action. After the confirmed goal completion, the compact idle state MUST
record `goal state=complete` and `plan state=idle`. It preserves the lane; it
MUST NOT claim the issue or implementation complete, poll, interrupt, duplicate,
replace, or approve the live producer or reviewer. The #590 and #609 lanes
waiting for #591's merged SHA illustrate this condition: their local work was
complete, while the merge remained an external producer.

A qualifying wake event MUST cause the child to create a fresh short-lived goal
and current plan before any edit, proof, review response, publication, or merge
work. `update_goal(blocked)` remains reserved for the typed unanswered
user-decision boundary.

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
bound fields. They MUST NOT require parent pre-delivery or post-result
registration receipts for initial `get_goal`, `create_goal`, or active readback;
native startup order remains governed by `goal-lifecycle`.
