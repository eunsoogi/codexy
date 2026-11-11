# Worker Creation Denial Recovery

Codex MUST use this procedure when an authorized parent operation to create an
app Worker receives denial feedback or its outcome is uncertain. This procedure
consumes the producer's existing guidance. It MUST NOT change host policy,
create task authority, or replace the current issue owner.

## Classify the observed result

Before retrying, Codex MUST bind the decision to the attempted operation, the
actual hook or tool response, the request fields and their value sources, any
needed task and owner readback, and whether the call was prevented, completed,
or remains uncertain. Producer diagnostics, actual host restrictions, and
uncertain transport outcomes MUST remain distinct. A
`CODEXY_CHILD_THREAD_CREATION_*` diagnostic identifies a Codexy hook result; it
MUST NOT by itself be treated as a host permission denial.

Codex MUST read the model-visible feedback in its event channel: for
`PreToolUse`, the `permissionDecisionReason` and `additionalContext` when
present; for `PermissionRequest`, `decision.message`. A code string or user-only
log MUST NOT be treated as the complete correction.

- `MISSING_ROUTE_FIELDS`, `MISSING_MODEL`, `MISSING_THINKING`,
  `UNSUPPORTED_MODEL`, and `UNSUPPORTED_THINKING` identify request-field
  problems. Codex MUST use the exact cause and the existing assignment as the
  correction source. For model or thinking fields, Codex MUST preserve the pair
  already selected for that assignment. If no pair was selected, Codex MUST
  apply the canonical Worker routing criteria to the available task evidence; it
  MUST NOT force the ordinary route for hard work or rely on host defaults.
- `APP_WORKTREE_REQUIRED` identifies the child creation target. Codex MUST use
  the actual `projectId` from the saved project selected for this task and set
  `target.type="project"` with `target.environment.type="worktree"` in the
  original `create_thread` call. Codex MUST NOT guess a project ID or retrofit a
  local or projectless task.
- `PARENT_WORKTREE` and `PARENT_CHECKOUT_UNVERIFIED` describe the caller's
  checkout, not child-request fields. Creation MUST remain pending until a
  different controlling task uses a supported handoff to the saved project's
  primary checkout and verifies its actual CWD there. The caller MUST NOT use
  `handoff_thread` to move itself or change the child target to hide the
  checkout problem.
- `CODEXY_CHILD_THREAD_CREATION_ENVELOPE`,
  `CODEXY_CHILD_THREAD_CREATION_RUNTIME`, and other producer/runtime failures do
  not identify a request-field correction. Codex MUST preserve the exact
  diagnostic and use the existing producer or host failure route. It MUST NOT
  relabel such a failure as a user decision or a route-field error.

A confirmed `PreToolUse` or `PermissionRequest` hook denial establishes that the
specific invocation was prevented. A timeout, interrupted call, missing response
after the tool ran, or other result that does not establish whether creation
occurred MUST remain uncertain. A denial label alone MUST NOT prove that no
child was created or that the host rejected the operation for a permission
reason.

## Retry after a verified correction

Codex MUST retry a denied creation only while the original assignment remains
authorized and one of these verified corrections applies:

- For a request-field denial, the feedback identifies a correctable field and an
  authoritative source provides its corrected value. Codex MUST change only the
  identified fields in the original request.
- For `PARENT_WORKTREE` or `PARENT_CHECKOUT_UNVERIFIED`, another controlling
  task completes the supported handoff. Authoritative task readback MUST confirm
  this same caller thread remains bound to the saved project selected for this
  task and its actual CWD is that project's primary checkout. This corrects a
  caller-side execution precondition; Codex MUST keep every field in the
  original creation request unchanged.

Both routes MUST preserve the issue, owner, saved project, app-managed worktree,
and selected model pair. Verified checkout recovery is a meaningful correction
even though the request fields remain unchanged. Codex MUST NOT retry while the
same failed request field or caller-checkout precondition remains unresolved, or
ask the user to choose routine model, thinking, or worktree settings.

If a required value is missing from every authoritative source, Codex MUST keep
only that creation operation pending and report the missing source. It MUST NOT
fabricate a value or widen task authority.

An actual host, organization, or tool restriction is not a malformed route
field. Codex MUST preserve the exact operation and returned restriction, then
report the supported owner or capability needed to resolve it. It MUST NOT
bypass the restriction with a local or projectless task, another account, a
native subagent, a default model, or repeated unchanged attempts. A new user
decision is needed only when the user stops or revokes the work, changes its
scope, or leaves a material choice unresolved.

A direct user stop MUST stop the covered creation operation immediately. Codex
MUST NOT resume or retry that operation until the user resumes it or grants
authority again. Codex MUST continue only independent work that remains
authorized by the user's instruction. Goal-state changes MUST follow the
[goal-lifecycle contract](../../../goal-lifecycle/SKILL.md); a stop MUST NOT be
used to manufacture completion or a blocked result.

## Resolve uncertain outcomes and preserve the owner

Before retrying a call whose outcome is uncertain, Codex MUST read the current
app task and owner state through supported task surfaces. It MUST inspect the
matching task or thread and use native goal readback where needed. Any
discovered child MUST be bound to the original request and issue lane; a task
listing that is empty, truncated, or unrelated does not establish that the child
was not created.

- If one matching child exists, Codex MUST retain it as the sole implementation
  owner and route continuation or corrections to that child. Codex MUST NOT
  create a replacement.
- If multiple matching children exist or ownership remains ambiguous, Codex MUST
  keep creation pending, preserve the exact owner and task identities, and
  reconcile ownership through the existing task/owner route. A non-owner MUST
  report the exact ambiguity to the controlling Orchestrator. Codex MUST NOT
  choose an owner arbitrarily or create another child.
- Codex MUST retry only after authoritative readback establishes that no child
  was created, the original assignment still authorizes creation, a verified
  correction above applies, and the supervised child reservation is reconciled.
  An empty, truncated, or unrelated task listing MUST NOT establish these
  conditions; one uncertain attempt MUST NOT allocate a second slot.
- After a permitted retry succeeds, Codex MUST continue the original issue
  handoff with the returned child as its owner. If the cause is resolved by a
  supported continuation, Codex MUST preserve the existing implementation
  owner's goal, branch, and worktree. If the creation caller itself must move to
  the primary checkout, it MUST use only the supported handoff; that MUST NOT
  transfer the issue lane or replace its existing owner. Codex MUST NOT reset
  the goal or create a replacement owner to resume the same assignment.

Codex MUST keep useful, independent authorized work moving while only the
affected creation remains pending. A denial MUST NOT itself transfer ownership
or change a native goal. A repeated denial or turn count alone MUST NOT mark a
goal blocked; native blocked status follows the
[goal-lifecycle contract](../../../goal-lifecycle/SKILL.md#deciding-whether-work-is-blocked).

## Evidence boundary

Hook source, denial text, tests, and semantic review MUST NOT be reported as
proof that the host invoked the hook, delivered its feedback to a model, or
observed a corrected next call. Codex MUST report those live behaviors as
unverified until request-correlated evidence from the actual host is available.
Source-level recovery guidance also MUST NOT be reported as native goal, owner,
or continuation proof.
