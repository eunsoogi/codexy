# Worker routing policy

This reference keeps the `child-routing-policy.md` filename for compatibility;
the product role described by the ordinary route is Worker.

Named packaged specialists are selected first and caller model overrides are
forbidden. Ordinary Worker work defaults to `gpt-6-luna` at `max`; when that
route is unavailable, it fails closed to the Orchestrator or named-specialist
route. Simple work uses the same Worker route when all simple predicates are
complete.

The native `create_thread` admission hook MUST enforce the Worker pair,
including rejecting omissions and caller-selected model or thinking changes
before mutation. The installed concern and launcher retain their
`child_thread_creation` identifiers for compatibility. Native specialists remain
a separate catalogued route with their assigned settings; a caller-written role
or prompt MUST NOT authorize a Worker override. Requested fields and
source-level hook admission do not prove effective host state; missing
observations MUST be recorded as unavailable/not observed, not claimed as
observed.

For bounded native observation of assigned Codex Workers, the Orchestrator MUST
select the packaged `codexy-watcher` specialist and summon it through the host's
native subagent facility. It MUST NOT substitute an unassigned subagent or treat
a self-declared role name as specialist identity. The packaged Watcher declares
`gpt-6-luna` at `max`; caller overrides remain forbidden.

Thread delivery MUST bind `model` and `thinking` to the authenticated recipient,
not copy the sender settings. The existing host-envelope directions
`root_to_child` and `child_to_parent` are serialized compatibility identifiers:
they represent Orchestrator-to-Worker delivery using `gpt-6-luna` at `max` and
Worker-to-Orchestrator delivery using `gpt-6-astra` at `medium`, respectively.
The runtime request's serialized `parent_to_generic` and `child_to_root`
directions remain unchanged for the same two routes. Both fields MUST be
explicit. Unsupported or mismatched recipient settings MUST fail closed instead
of falling back to the sender route.

Worker selection owns recipient and model routing, not verification policy. The
closed route in [context tiers](context-tiers.md) selects profile, sequencing,
finite-work, review, and final-proof references only when applicable.
Collaboration shape requires ownership metadata where the contract says so, but
MUST NOT by itself select a strict profile or historical-review path.

## Review routing

For a Worker-owned implementation lane, the owning Worker MUST delegate the
profile-selected internal reviewer after local proof when the selected profile
requires one. The Orchestrator MUST consume that evidence and MUST NOT invoke
the internal reviewer or tell the Worker not to invoke it. This is distinct from
a repository-required external `@codex review`, which the Orchestrator requests
when applicable; the Worker MUST NOT request that external review.

Author self-review is forbidden, but delegating the independent packaged
reviewer is not self-review. The nonrecursive prohibition for a helper or
reviewer
(`MUST NOT spawn, delegate to, or create any additional agent, helper,
reviewer, task, or thread.`)
applies to that recipient and MUST NOT be copied into a Worker assignment as a
ban on the Worker's required review.

Control-plane receipts and status handoffs MUST carry a stable `transition key`,
`event id`, or `state fingerprint` as applicable. Pre-delivery and post-result
receipts MUST use `transition key`. A completed delivery with the same
recipient, phase, and key MUST NOT be sent again; unchanged status MUST remain
silent.

Unknown, ambiguous, incomplete, and unsupported requests fail closed to the
Orchestrator or named-specialist route. The executable contract is maintained by
the packaged runtime validator.
