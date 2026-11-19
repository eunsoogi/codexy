# Supervision reservation store contract

`codexy_policy.supervision_reservations.Reservations` is a local transactional
store interface for the parent-owned supervision record. The caller MUST choose
a private persistent database path and authenticate the parent, request, issue,
and tool observations. This module does not discover children, authenticate hook
data, dispatch work, or release capacity automatically. Its presence alone does
not enforce an app-child limit.

A reservation is keyed by `(parent, request)` and retains one issue, requested
model and thinking pair, optional pending setup ID, and optional child ID. The
parent identifier is supplied by the trusted caller; an app task listing MUST
NOT be used to infer it. The issue remains owned by the same request after
release so a later resume cannot silently create a replacement owner. An
existing child can be looked up by `(parent, child)` through `lookup`, which
returns the retained route and state. Consumers MUST NOT infer an effective
model from the requested pair.

- `reserve(parent, request, issue, model, thinking, limit=8)` atomically admits
  one slot or returns `queued`. An already reserved request returns `existing`;
  an existing child admitted for another turn returns `resumed`. Neither result
  authorizes creating another child. Competing owners for the same issue fail.
  The effective ceiling is the smallest of eight and any lower limit supplied
  for that parent so far; an omitted later limit cannot raise it. Resumption of
  a released request reserves capacity again before reactivating its old owner.
- `setup(parent, request, pending)` binds a setup ID to the existing slot.
  `ready(parent, request, child)` binds the trusted ready result to that same
  slot and preserves its requested pair. Conflicting identities fail.
- `adopt(parent, request, issue, model, thinking, child)` records an already
  known owner from trusted evidence. It preserves owners even above the ceiling;
  subsequent reservations remain queued until capacity is available.
- `release(parent, request, confirmed=True)` frees the slot but retains the
  ownership record. The caller MUST pass confirmation only after verifying
  termination and absence of remaining execution, or confirmed failed setup with
  no live owner. Timeout, idle, an archive request, an omitted listing, or an
  ambiguous response MUST NOT be treated as confirmation.

All changes serialize using a database write transaction. A queued request is
not stored, so the parent MUST retain the backlog and decide eligibility. The
store does not discover children created before it was introduced: the caller
MUST reconcile existing owners before admitting new work. Until the installed
app provides a verified event and correlation route, the store and its tests do
not prove that create, resume, or delivery calls are enforced.

`supervision_dispatch.Dispatch.prepare` reserves the canonical SHA-256 digest of
the exact structured creation input for an explicit issue and its requested
model pair. `claim` binds that digest and parent session to a native tool call
ID in one transaction. A second call cannot consume the same reservation;
repeated delivery of the same call is idempotent. Confirmed release retains a
closed-call tombstone so a late event cannot claim a later attempt. No raw
prompt is stored in this mapping. Callers MUST NOT infer the issue or pair from
free-form prompt or title text.

The provisional `supervision_observation` adapter consumes trusted hook events.
`admit_create` requires a matching prepared input before allowing creation;
`observe_create` reconciles the corresponding `PostToolUse` result by session,
call ID, and input digest. It accepts an unambiguous ready or pending MCP result
and keeps unknown, failed, malformed, or conflicting results reserved. It is is
registered for creation events. The `codexy_supervision_prepare.py` accepts an
explicit parent session, issue, lower limit, private plugin-data root, and the
exact structured creation input on standard input. The hook entrypoint uses the
host-provided `PLUGIN_DATA` and rejects missing or non-private state. Callers
MUST reconcile all existing owners before preparing new dispatches. The host
route for exposing that same plugin-data root to the preflight caller, runtime
event delivery, and installed-app behavior remain unverified. The documented
event fields and hook coverage are described in the
[Codex hooks reference](https://developers.openai.com/codex/hooks).

Before the first dispatch in a parent session, the Orchestrator MUST reconcile
the complete existing owner roster and pass the resulting JSON array to
`codexy_supervision_owner.py --plugin-data <root> --parent <session> bootstrap
--confirmed-complete`.
Each element MUST contain `request`, `issue`, `model`, `thinking`, and `child`
from known assignments; an empty array is valid only when no supervised children
exist. The hook denial supplies the host-provided plugin-data root when it is
otherwise unavailable. Before each new creation, the Orchestrator MUST supply
the exact native tool-input JSON on standard input to
`codexy_supervision_prepare.py --plugin-data <root> --parent <session>
--issue <issue>`,
including any explicit lower `--limit`. A queued or existing result MUST NOT
authorize a new creation. After the exact creation call, the hook retains
uncertain outcomes until actual setup and owner evidence is known.

For a confirmed terminated owner with no remaining execution, the parent MAY use
`codexy_supervision_owner.py --plugin-data <root> --parent <session> release
--child <child> --confirmed`.
The flag records the caller's verified conclusion; it is not itself proof.
Reserve the same owner with `resume --child <child>` before resuming it. The
current implementation does not intercept resume calls; direct delivery
enforcement remains unverified.
