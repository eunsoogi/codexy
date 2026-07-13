# Parent Goal Transition Reporting

## Scope

This is the static evidence and instruction contract for delegated child goal
operations. Issue #367 owns runtime task delivery; Issue #373 owns runtime
deduplication, restart recovery, worktree preservation, and replacement.

## Source Parent Binding

A delegated child with `source_thread_id` MUST record that exact value as the
source parent Codex task id in lane control state. It MUST use the actual Codex task/thread messaging surface to contact that id. Local multi-agent messaging,
including `agents.send_message('/root')`, MUST NOT be presented as a substitute.

Each receipt MUST carry a stable transition key. A static fixture MUST use the
same source task id and transition key for its pre-delivery, goal call, and
post-result records. Repeated delivery evidence for one key MUST be represented
as deduplicated; it MUST NOT imply a second goal call.

## Ordered Receipts

Before `create_goal`, `update_goal(complete)`, or `update_goal(blocked)`, the
child MUST send a compact intended-transition delta to its source parent. The
pre-delivery receipt MUST name issue/PR, pending goal action or objective,
parent task id, current plan step, branch, worktree, HEAD, dirty/index state,
evidence, next action, stable transition key, and confirmed task-surface
delivery.

After every goal tool call, including `get_goal`, the child MUST send a
post-result receipt containing the exact tool result, operation, parent task
id, matching transition key, and confirmed task-surface delivery. A prose-only
claim that delivery or a result happened is not a receipt.

`update_goal(blocked)` MUST NOT execute until parent delivery is confirmed. If
the delivery is unavailable, static evidence MUST show one terminal
parent-messaging-unavailable receipt and no blocked goal call. The runtime
delivery mechanics remain owned by #367.

## Static Recovery Shapes

Static validator fixtures MUST cover representative handoff shapes: #360 and
#276 blocked notices, #311 and #365 usage-limited notices, and #350 task-CWD
versus canonical reserved worktree mismatch. These are evidence-contract
fixtures only; they MUST NOT claim runtime allocator, archive, replacement, or
freeze behavior owned by #373.

When a fixture shows a task CWD that differs from the canonical reserved
worktree, it MUST report the mismatch before any goal continuation evidence.

## Validator Contract

The static validator MUST reject missing pre-delivery, missing post-result,
reversed ordering, wrong parent ids, local-agent routing, missing required
pre-delivery fields, prose-only claims, duplicate goal calls for one transition
key, and blocked calls before confirmed parent delivery.
