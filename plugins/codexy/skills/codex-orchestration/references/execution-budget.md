# Finite Child Execution Budgets

Every non-trivial child lane MUST declare a finite execution budget before edits begin.
The budget MUST name finite implementation, repair, and reviewer cycle limits.
Continuation MUST consume budget and record either an explicit acceptance criterion newly satisfied or an existing blocker removed.

File, diff, test, or fingerprint churn without reducing remaining acceptance work MUST NOT renew or reset the budget.
A renewal MUST be an explicit parent-owned new finite budget with recorded acceptance progress or blocker removal.
A child MUST NOT self-renew from changed artifacts alone.

After all acceptance criteria and required proof are complete, the lane MUST terminate implementation; adjacent findings become non-blocking follow-up candidates.
Budget exhaustion MUST produce one compact terminal parent handoff with current goal/plan, branch/worktree/HEAD, dirty inventory, proof, remaining criteria, and recommended next decision.

Budget exhaustion MUST NOT call `update_goal(blocked)` and MUST NOT weaken external-gate heartbeat semantics.
An external parent heartbeat MUST observe waiting state without messaging the child and MUST send one continuation only on a material transition.
Repeated child waiting turns, goal refreshes, polling, duplicate narrative, unbounded reasoning, or status-only parent receipts MUST consume budget and MUST NOT qualify as acceptance progress.
The execution-budget contract MUST apply to GPT-5.6 Terra child lanes while remaining model-agnostic and MUST NOT hard-code model-specific prose into the state machine.
