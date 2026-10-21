# Worker routing policy

This reference keeps the `child-routing-policy.md` filename for compatibility;
the product role described by the ordinary route is Worker.

Named packaged specialists MUST be selected when their role matches the task;
their catalogued model and effort apply automatically, and callers MUST NOT
override them. Ordinary Worker selection is difficulty based: a clearly scoped
general implementation task uses `gpt-6.1-sol` at `medium`; a simple task uses
`gpt-6-luna` at `max` only when all four `simple_predicates` are true.
Difficulty uncertainty may support a general classification, but ambiguous
scope, unknown authority, or missing required inputs MUST stay fail-closed as
`ambiguous`, `incomplete`, or `high_risk`. Unsupported model or reasoning
capabilities MUST keep the route pending with an explicit safe outcome. Codex
MUST NOT ask the user to choose or reconfirm a defined pair. If the selected
host or tool cannot honor it, Codex MUST report the exact limitation and keep
the lane pending; it MUST NOT silently substitute a model, sender settings,
parent implementation, or duplicate Worker. A specialist is a separate route
only when its role matches the task, not a fallback for model unavailability.

The native `create_thread` admission hook MUST enforce the selected Worker pair
and the app-managed project worktree environment before mutation. Every Worker
call MUST include the selected `model` and `thinking` values and set
`target.type="project"`, a non-empty `projectId`, and
`target.environment.type="worktree"` in the original call. Local or projectless
Worker requests MUST be denied. A later `create_worktree`, `fork_thread`, shell
`git worktree add`, or detached directory MUST NOT be used as a substitute for
the initial task environment. If the actual host contract cannot accept the pair
or worktree environment, Codex MUST keep the lane pending and report the exact
incompatibility; it MUST NOT omit a field or rely on defaults. The installed
concern and launcher retain their `child_thread_creation` identifiers for
compatibility. Native specialists remain a separate catalogued route with their
assigned settings; a caller-written role or prompt MUST NOT authorize a Worker
override. Requested fields and source-level hook admission do not prove
effective host state; missing observations MUST be recorded as unavailable/not
observed, not claimed as observed.

For ordinary observation of assigned Codex Workers, the Orchestrator MUST use
the native event-driven `wait_threads` route with each task's latest cursor. It
MUST NOT summon a Watcher solely to relay routine Worker events. A separately
assigned bounded observation MAY use the packaged `codexy-watcher` specialist,
summoned through the host's native subagent facility; only that Watcher may
observe its exact assigned targets, and the Orchestrator MUST await
`watcher_wait` without also waiting directly on them. It MUST NOT substitute an
unassigned subagent or treat a self-declared role name as specialist identity.
The packaged Watcher declares `gpt-6-luna` at `max`; caller overrides remain
forbidden.

When creating a Worker, Codex MUST bind the selected model and effort to the
authenticated recipient. For existing tasks, Codex MUST send to the
authenticated `threadId` with the recipient's assigned model and effort; it MUST
NOT copy sender settings. If the actual message-tool contract cannot accept that
pair, Codex MUST NOT send an omitted or partial call or rely on defaults; it
MUST preserve the lane as pending and report the exact incompatibility. The
existing host-envelope directions `root_to_child` and `child_to_parent` are
serialized compatibility identifiers. `root_to_child` represents
Orchestrator-to-Worker delivery using the authenticated recipient's assigned
Worker pair (`gpt-6-luna` at `max` or `gpt-6.1-sol` at `medium`); ordinary
Worker creation selects Luna/Max for simple work with all four predicates and
Sol/Medium for clearly scoped general work. `child_to_parent` represents
Worker-to-Orchestrator delivery using `gpt-6-luna` at `max`, the same pair used
by simple Workers. Authenticated direction and recipient identity distinguish
the parent recipient; the pair alone does not. The runtime request's serialized
`parent_to_generic` and `child_to_root` directions remain unchanged for these
delivery routes. These serialized identifiers do not authorize parameters that
the actual tool contract forbids. Unsupported or mismatched recipient settings
MUST leave the lane pending with the exact limitation. Missing observations MUST
be reported as unavailable/not observed. Codex MUST NOT fall back to the sender
route.

The runtime keeps `codexy.child-routing-request.v1` for thread creation,
Worker-to-Orchestrator delivery, and other existing routing requests. A v1
Orchestrator-to-Worker follow-up has no recipient assignment, so the resolver
MUST leave it pending with a migration reason instead of inferring settings.
Version 2 is reserved for `send_message_to_thread` requests in the
`parent_to_generic` direction. It MUST include the recipient's exact
`model`/`thinking` pair under `recipient_assignment`; both values MUST be an
allowed Worker pair and MUST be advertised by the declared thread capabilities.
Top-level sender `model`/`thinking` fields MUST be rejected. The resolver's
selected pair MUST be used for that authenticated recipient, and the delivery
hook MUST independently reject crossed, unsupported, or mismatched arguments.
The selected pair MUST pass through follow-up routing unchanged.

### Blocked-goal fork recovery

An authorized same-directory fork continues the source task lane and role; it is
not a new Worker or role assignment. For that same task, this recovery exception
MUST preserve the source model and reasoning effort even when they differ from
ordinary role defaults. Codex MUST read requested values from the source
assignment or creation record and keep host-exposed effective values separate.
For each field, Codex MUST use the effective source value when observable and
otherwise the exact requested value. If either is unavailable from both sources,
Codex MUST keep the lane pending. The continuation MUST use
`send_message_to_thread` with explicit `model` and `thinking` fields; it MUST
NOT inherit sender or role defaults or reselect the task role. A requested pair
proves only what was requested. If effective settings are not exposed, Codex
MUST report them as unavailable/not observed, not as preserved.

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

When a handoff contract requires a receipt, it MUST carry that contract's stable
`transition key`, `event id`, or `state fingerprint`. A completed delivery with
the same recipient, phase, and key MUST NOT be sent again; unchanged status MUST
remain silent. Initial `get_goal`, `create_goal`, and the required active
readback MUST happen in the owning task before task work, but MUST NOT generate
separate parent registration receipts. Blocked-goal recovery and terminal
handoff receipts remain governed by their canonical references.

Unknown, ambiguous, incomplete, and unsupported requests fail closed to the
Orchestrator or named-specialist route. The executable contract is maintained by
the packaged runtime validator.
