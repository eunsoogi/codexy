# Worker routing policy

This reference keeps the `child-routing-policy.md` filename for compatibility;
the product role described by the ordinary route is Worker.

## Orchestration-request dispatch

For an orchestration request, MUST apply the
[ownership boundary](task-classification.md#orchestration-request-ownership)
before dispatch, including independent read-only work. MUST reuse an existing
suitable child owner. Otherwise, MUST select an authorized current host route: a
matching bounded specialist helper for research/review without its own branch or
PR, or the app-managed task/worktree route for an issue implementation owner. A
helper MUST NOT replace, duplicate, or patch an implementation owner. Ordinary
single-task requests MUST NOT acquire mandatory delegation from this policy.

Each assignment MUST name the exact objective, scope, input evidence,
deliverable and success criteria, verification, and stop/report conditions.
Creating a child only to report status while the parent does the substantive
work MUST NOT count as delegation. The parent MUST evaluate actual child work
and returned evidence; a prompt or task-creation result alone MUST NOT prove
execution.

Every app-managed child assignment governed by a native goal MUST explicitly
direct the child to read the
[goal-lifecycle skill](../../goal-lifecycle/SKILL.md), call native `get_goal`
before any task-specific work, and complete the skill's required first
transition. The assignment MUST make clear that `get_goal` precedes issue or
repository reads, commands, verification, delegation, and edits; later goal
activation MUST NOT satisfy the required ordering.

An explicit user assignment to orchestrate a defined outcome, including
directing non-orchestration work to children, authorizes the necessary bounded
child tasks and assignment, follow-up, reporting, and recovery messages within
that scope. Codex MUST NOT ask for approval again per child or per message. This
authority MUST NOT expand scope, override explicit no-new-task or recipient
limits, bypass host/organization permissions, or authorize merge, release,
publication, or destructive actions. Host denial MUST preserve the exact
response and assignment; Codex MUST NOT invent permission parameters, bypass the
host, or ask the user to repeat authorization already given. Ordinary questions
and vague coordination requests MUST NOT create this authority. MUST follow the
[thread/worktree rules](thread-and-worktree-routing.md). Before app-child
creation or resume, MUST reconcile ownership and reserve capacity under the
[supervised eight-slot ceiling](parent-supervision.md#supervised-app-child-capacity)
and applicable helper budget. At capacity, unstarted work MUST stay in backlog.
If the required child surface or authority is unavailable, MUST report the exact
limitation and required owner decision and keep the work pending; parent
execution, an unsupported CLI/app-server route, or another owner MUST NOT be a
fallback. A blocked child MUST follow the
[Advisor-assisted supervision route](parent-supervision.md#waiting-and-direct-correction);
the parent MUST retain the owner and direct recovery rather than do its work.

## Recipient routing

Named packaged specialists MUST be selected when their role matches the task;
their catalogued model and effort apply automatically, and callers MUST NOT
override them. Ordinary, bounded, and simple Worker assignments MUST default to
`gpt-6-luna` / `max`. A new general Worker with valid hard-work facts MUST be
assigned `gpt-6.1-terra` / `medium` directly; the `general` classification alone
MUST NOT select that pair. Existing Sol/Medium recipients remain eligible for
authenticated follow-up delivery but MUST NOT be selected for new Worker
creation. For general work, `fixed_scope`, `low_risk_reversible`, and
`no_unresolved_decision` MUST be true; `deterministic_oracle` MUST be supplied
but MAY be false. Missing safety facts, ambiguous scope or authority, missing
inputs, and unresolved ownership MUST stay fail-closed. Difficulty evidence MUST
NOT substitute for those decisions. Unsupported model or reasoning capabilities
MUST keep the route pending with an explicit safe outcome. Codex MUST NOT ask
the user to choose or reconfirm a defined pair. If the selected host or tool
cannot honor it, Codex MUST report the exact limitation and keep the lane
pending; it MUST NOT silently substitute a model, sender settings, parent
implementation, or duplicate Worker. A specialist is a separate route only when
its role matches the task, not a fallback for model unavailability.

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
the native event-driven `wait_threads` route with each task's latest cursor. It
MUST NOT summon a Watcher solely to relay routine Worker events. A separately
assigned bounded observation MAY use a Watcher supplied outside Codexy's
packaged specialist catalog. Only that Watcher may observe its exact assigned
targets, and the Orchestrator MUST await `watcher_wait` without also waiting
directly on them. The core plugin no longer registers Watcher MCP or ships the
retired launcher and dedicated bootstrap. A host using an external compatibility
transport must supply a compatible entrypoint; the packaged catalog does not
include a native Watcher specialist or model pair.

When creating a Worker, Codex MUST bind the selected model and effort to the
authenticated recipient. For existing tasks, Codex MUST send to the
authenticated `threadId` with the recipient's assigned model and effort; it MUST
NOT copy sender settings. If the actual message-tool contract cannot accept that
pair, Codex MUST NOT send an omitted or partial call or rely on defaults; it
MUST preserve the lane as pending and report the exact incompatibility. The
existing host-envelope directions `root_to_child` and `child_to_parent` are
serialized compatibility identifiers. `root_to_child` represents
Orchestrator-to-Worker delivery using the authenticated recipient's assigned
Worker pair (`gpt-6-luna` at `max`, `gpt-6.1-terra` at `medium`, or an existing
`gpt-6.1-sol` at `medium`); ordinary Worker creation defaults to Luna/Max,
including general implementation, and valid hard-work facts select Terra/Medium
for a new Worker. Sol/Medium is delivery compatibility for existing recipients
only. For a valid `root_to_child` `PreToolUse` delivery to an existing
Sol/Medium recipient, the thread-delivery hook may add a deprecation notice to
the sending Orchestrator's `additionalContext`. The Orchestrator MUST preserve
the recipient, model, thinking, and prompt and MUST NOT switch, fork, or resend
solely because of that advisory. The warning MUST NOT approve a denial from
another hook, and `PermissionRequest` remains quiet. `child_to_parent`
represents Worker-to-Orchestrator delivery using `gpt-6.1-sol` at `low` (Sol /
Light), the requested main Orchestrator pair. Stale parent Luna/Max and
Luna/Medium pairs MUST be rejected on this authenticated direction.
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

Only a general Worker MAY receive a direct Terra/Medium assignment by supplying
`hard_work` with one closed `criterion` and its complete facts:

| Criterion                  | Required evidence                                                                                                                                                                                                    |
| -------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `coupled_contracts`        | `boundaries`: at least three distinct executable contracts that must change together; `coupling`: why changing one independently would violate the shared invariant.                                                 |
| `algorithm_design`         | `problem`: a new algorithm to derive; `constraints`: at least two distinct technical constraints; `correctness_obligation`: the invariant or proof needed beyond applying an existing pattern.                       |
| `cross_boundary_diagnosis` | `boundaries`: at least two affected technical boundaries; `competing_hypotheses`: at least two distinct plausible causes; `observations`: at least two concrete observations that discriminate between those causes. |

The assigning owner MUST verify these facts against supplied task evidence.
Lists MUST contain distinct nonempty facts; required prose MUST be nonempty. The
parser validates structure, not semantic truth. File count, a general
implementation label, an unfamiliar library, long prose, or model availability
alone MUST NOT count as evidence. Routine edits, bounded summaries,
known-pattern implementations and single-boundary fixes retain Luna/Max. A
malformed, unknown, or incomplete hard-work object MUST be rejected; missing
hard-work evidence MUST NOT cause escalation. Valid evidence MUST NOT change a
simple, bounded, ambiguous, incomplete, or high-risk request into a general
assignment. Capability checks MUST apply to the selected exact pair without
substitution.

This is the direct assignment path for work already evidenced as hard. When a
general Luna/Max Worker discovers qualifying evidence after work begins, the
Orchestrator MUST use the [Worker elevation decision](worker-elevation.md); the
Orchestrator MUST NOT treat that assessment as executing a fork or changing the
existing Worker's model or ownership. Before any source/target handoff, the
parent MUST also follow the
[Worker elevation reconciliation](worker-elevation/reconciliation.md) and keep
the existing owner until the target setup and supported retirement are
confirmed.

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
