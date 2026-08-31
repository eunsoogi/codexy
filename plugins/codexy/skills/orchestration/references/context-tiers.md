# Context retention and safety

The runtime retains safety state on every handoff: issue and PR identity,
owner and worktree, base and head, dirty-index state, checks, unresolved review
threads, selected reviewer state, verification, external gates, and the next
action. Omitted safety state is typed and never treated as proof of absence.

Task references are selected from the closed task, surface, and risk routes in
the runtime handoff contract. Unknown, ambiguous, high-risk, security,
permission, and release classifications fail closed through `child_routing`.

Stable handoff identity covers the workflow classification and selected
references. Volatile identity covers the current safety and verification state.
Full conversation, full tool bodies, and full agent trees are never forwarded.

The executable route and retention contract is maintained by the packaged
runtime validator.
