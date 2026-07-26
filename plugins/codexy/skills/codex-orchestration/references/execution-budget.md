# Finite Child Execution Budgets

Every non-trivial child lane MUST declare a finite execution budget before edits begin.
The budget MUST name finite implementation, repair, and reviewer cycle limits.
Continuation MUST consume budget and record either an explicit acceptance criterion newly satisfied or an existing blocker removed.

Every non-trivial parent-owned orchestration stage MUST declare finite implementation, repair, fanout, and reviewer-cycle limits before work begins.
A parent-owned stage MUST NOT use more than three non-Sentinel specialists in total; the packaged Sentinel remains separate.
A repeated parent helper or reviewer cycle MUST record either an explicit acceptance criterion newly satisfied or an existing blocker removed.
Unchanged wait output and full-state replay MUST consume the parent-stage budget and MUST NOT renew implementation, repair, fanout, or reviewer-cycle limits.
A bounded thread-read fallback that returns oversized preview or history output MUST consume the current parent-stage budget, MUST record only bounded size and token metadata, and MUST NOT renew the stage.
Parent-stage budget enforcement MUST preserve external-wait heartbeat semantics and the packaged Sentinel review gate.

File, diff, test, or fingerprint churn without reducing remaining acceptance work MUST NOT renew or reset the budget.
A renewal MUST be an explicit parent-owned new finite budget with recorded acceptance progress or blocker removal.
A child MUST NOT self-renew from changed artifacts alone.

After all acceptance criteria and required proof are complete, the lane MUST terminate implementation; adjacent findings become non-blocking follow-up candidates.
Budget exhaustion MUST produce one compact terminal parent handoff with current goal/plan, branch/worktree/HEAD, dirty inventory, proof, remaining criteria, and recommended next decision.

Budget exhaustion MUST NOT call `update_goal(blocked)` and MUST NOT weaken external-gate heartbeat semantics.
An external parent heartbeat MUST observe waiting state without messaging the child and MUST send one continuation only on a material transition.
Repeated child waiting turns, goal refreshes, polling, duplicate narrative, unbounded reasoning, or status-only parent receipts MUST consume budget and MUST NOT qualify as acceptance progress.
The execution-budget contract MUST apply to GPT-5.6 Terra child lanes while remaining model-agnostic and MUST NOT hard-code model-specific prose into the state machine.
