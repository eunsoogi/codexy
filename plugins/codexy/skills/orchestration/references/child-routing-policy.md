# Worker routing policy

This reference keeps the `child-routing-policy.md` filename for compatibility;
the product role described by the ordinary route is Worker.

## Orchestration-request dispatch

For task kind, scoped authority, implementation owner, and the distinction
between dependencies, resource reservations, and integration conflicts, MUST
follow
[task classification](task-classification.md#orchestration-request-ownership).
That decision MUST precede dispatch; this reference MUST NOT create a competing
decision. For child-owned issue implementation or repair, MUST route the first
assignment and later corrections to its app Worker in the required managed
worktree. A matching native helper MAY perform bounded read-only research or
review; it MUST NOT replace, duplicate, or patch the implementation owner. If
that owner or its evidence is unavailable, MUST report the gap rather than
substitute a helper or parent implementation. MUST apply scoped user overrides
only as directed.

Each assignment MUST name the exact objective, scope, input evidence,
deliverable and success criteria, verification, and stop/report conditions.
Creating a child only to report status while the parent does the substantive
work MUST NOT count as delegation. A prompt or task-creation result alone MUST
NOT prove execution.

Every app-managed child assignment governed by a native goal MUST explicitly
direct the child to read the
[goal-lifecycle skill](../../goal-lifecycle/SKILL.md), call native `get_goal`
before any task-specific work, and complete the skill's required first
transition. The assignment MUST make clear that `get_goal` precedes issue or
repository reads, commands, verification, delegation, and edits; later goal
activation MUST NOT satisfy the required ordering.

After classification selects an owner and route, MUST follow the
[thread/worktree rules](thread-and-worktree-routing.md). Before child creation
or resumption, MUST reconcile capacity under the
[supervised-child ceiling](parent-supervision.md#supervised-app-child-capacity)
and applicable helper budget. At capacity, unstarted work MUST stay in backlog.
A blocked child MUST follow the
[Advisor-assisted supervision route](parent-supervision.md#waiting-and-direct-correction);
recovery MUST stay with its assigned owner.

## Recipient routing

For a required setting that conflicts with a host tool, MUST apply the
[skill and host conflict decision](../SKILL.md#skill-and-host-conflict-decision)
before choosing a supported route or reporting the affected operation pending.
This does not change the selected model, effort, or ownership rules below.

Named packaged specialists MUST be selected when their role matches the task;
their catalogued model and effort apply automatically, and callers MUST NOT
override them. Ordinary, bounded, and simple Worker assignments MUST default to
`gpt-6-luna` / `max`. For a general Worker, Luna/Max is also the default once
the safety facts pass; only complete, verified hard-work facts can select
Terra/Medium. A new general Worker with valid hard-work facts MUST be assigned
`gpt-6.1-terra` / `medium` directly; the `general` classification alone MUST NOT
select that pair. Authenticated follow-up delivery accepts only Luna/Max and
Terra/Medium, matching the Worker creation allowlist. Existing Sol/Medium
recipients MUST be rejected under authenticated routing metadata; Codex MUST NOT
silently reassign or migrate them. For general work, `fixed_scope`,
`low_risk_reversible`, and `no_unresolved_decision` MUST be true;
`deterministic_oracle` MUST be supplied but MAY be false. Missing safety facts,
ambiguous scope or authority, missing inputs, and unresolved ownership MUST stay
fail-closed. Difficulty evidence MUST NOT substitute for those decisions.
Unsupported model or reasoning capabilities MUST keep the route pending with an
explicit safe outcome. Codex MUST NOT ask the user to choose or reconfirm a
defined pair. If the selected host or tool cannot honor it, Codex MUST report
the exact limitation and keep the lane pending; it MUST NOT silently substitute
a model, sender settings, parent implementation, or duplicate Worker. A
specialist is a separate route only when its role matches the task, not a
fallback for model unavailability.

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
override. Requested settings and source-level admission do not prove effective
host state; unexposed settings MUST remain unavailable. A policy or installed
update MUST NOT imply that a running task changed or prove comparative
performance.

For ordinary observation of assigned Codex Workers, the Orchestrator MUST use
the native event-driven `wait_threads` route with each task's latest cursor.
Every returned target MUST retain its returned cursor for the next wait, while
omitted targets MUST keep their prior cursor. The Orchestrator MUST deduplicate
each event by stable identity before changing lane state. Empty, partial, or
truncated observations MUST remain unknown and MUST NOT establish that an event
or callback is absent. A native reviewer's terminal delivery follows its review
route and MUST NOT be confused with Worker observation. See
[observation evidence](observation-evidence.md) for absence claims.

When creating a Worker, Codex MUST bind the selected model and effort to the
authenticated recipient. For existing tasks, Codex MUST send to the
authenticated `threadId` with the recipient's assigned model and effort; it MUST
NOT copy sender settings. If the actual message-tool contract cannot accept that
pair, Codex MUST NOT send an omitted or partial call or rely on defaults; it
MUST preserve the lane as pending and report the exact incompatibility. The
existing host-envelope directions `root_to_child` and `child_to_parent` are
serialized compatibility identifiers. `root_to_child` represents
Orchestrator-to-Worker delivery using the authenticated recipient's assigned
Worker pair (`gpt-6-luna` at `max` or `gpt-6.1-terra` at `medium`), the same
allowlist as new Worker creation. Ordinary Worker creation defaults to Luna/Max,
including general implementation, and valid hard-work facts select Terra/Medium
for a new Worker. Existing Sol/Medium recipients MUST be rejected for
authenticated follow-up delivery; Codex MUST NOT reassign or migrate them.
`child_to_parent` represents Worker-to-Orchestrator delivery using `gpt-6.1-sol`
at `low` (Sol / Light), the requested main Orchestrator pair. Stale parent
Luna/Max and Luna/Medium pairs MUST be rejected on this authenticated direction.
Authenticated direction and recipient identity MUST remain enforced; recognizing
a model pair alone MUST NOT authorize delivery. The runtime request's serialized
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

### Bounded evidence summaries

When assigning bounded evidence work, MUST read the
[bounded evidence summaries](child-routing-policy/bounded-evidence-summaries.md)
for its exact limits, examples, and fallback behavior.

### Evidenced hard work

For the criteria, evidence requirements, and elevation boundary, MUST read
[evidenced hard work](child-routing-policy/evidenced-hard-work.md).

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

Recovery of the unchanged objective and scope carries the original assignment
authority; blocked status alone MUST NOT require new user confirmation. The
same-directory fork, necessary work preservation, actual CWD and Git check,
source-chat archive, and post-archive acceptance steps are owned by
[goal-lifecycle](../../goal-lifecycle/SKILL.md) and its
[executable-checkout recovery](../../goal-lifecycle/references/executable-checkout-recovery.md).
An unavailable artifact listing or separate lifetime guarantee MUST NOT block a
confirmed checkout. If a continuation remains blocked, the owner MUST apply the
same sequence from that current owner. This routing clause MUST NOT be read as a
separate recovery procedure.

Worker selection owns recipient and model routing, not verification policy. The
closed route in [context tiers](context-tiers.md) selects profile, sequencing,
finite-work, review, and final-proof references only when applicable.
Collaboration shape requires ownership metadata where the contract says so, but
MUST NOT by itself select a strict profile or historical-review path.

Child titles MUST follow the [title policy](child-thread-titles.md).

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

Assignments and corrections MUST use readable Markdown tables that convey the
purpose, scope, completion criteria, and any relevant ownership or direction.
Worker progress and results MUST use tables conveying status, changes,
verification, remaining work, and the next owner as applicable. One table MAY
cover multiple tasks when each task remains unambiguous. MUST NOT require a
second authored machine payload or hide one in a table cell.

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
