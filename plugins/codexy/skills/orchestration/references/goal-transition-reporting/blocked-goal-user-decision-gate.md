# Blocked Goal User-Decision Gate

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
