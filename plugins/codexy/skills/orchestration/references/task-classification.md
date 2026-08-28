# Task classification

MUST use the closed task, surface, and risk sets. Choose `light` for clear small
work, `standard` for non-trivial single-owner work, and `strict` for high-risk,
release, merge-sensitive, delegated, multi-lane, or audited work. Executable
boundaries need faithful RED/GREEN; non-engineering work needs readback, never a
manufactured RED.

Owner is `parent-owned`, `child-owned`, `current-thread-owned`, or
`external/human-owned`. A delegated branch, worktree, or PR stays child-owned;
parent retains orchestration and merge. Subagents are not worktree owners.
Compaction uses current-state authority.

Resolve shape-changing ambiguity before setup. Otherwise retain profile, owner,
scope, proof, blocker, and first action. MUST stop on missing scope, ownership
conflict, or unavailable authority. Clear work needs no table, N/A inventory,
clarification, skip list, or tool receipt.
