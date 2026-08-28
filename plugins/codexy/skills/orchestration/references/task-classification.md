# Task classification

MUST use the contract's closed sets. Use `light` for small/read-only,
`standard` for non-trivial single-owner, and `strict` for risk, release, merge,
delegation, multi-lane, or audit. Executable work needs faithful RED/GREEN;
other work needs readback, never a manufactured RED.

Owner is `parent-owned`, `child-owned`, `current-thread-owned`, or
`external/human-owned`. Delegated branch/worktree/PR stays child-owned; parent
keeps orchestration/merge; subagents are not owners. Compaction uses current
authority.

Before setup, resolve ambiguity changing scope, owner, risk, or external state.
Otherwise keep profile, owner, scope, proof, blocker, and next action. MUST stop
for missing scope, owner conflict, or unavailable authority. Clear work needs no
table, N/A, clarification, skip list, or tool receipt.
