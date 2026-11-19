# Supervision reservation store contract

`codexy_policy.supervision_reservations.Reservations` is a local transactional
store interface for the parent-owned supervision record. The caller MUST choose
a private persistent database path and authenticate the parent, request, issue,
and tool observations. This module does not discover children, authenticate
hook data, dispatch work, or release capacity automatically. Its presence alone
does not enforce an app-child limit.

A reservation is keyed by `(parent, request)` and retains one issue, requested
model and thinking pair, optional pending setup ID, and optional child ID. The
parent identifier is supplied by the trusted caller; an app task listing MUST
NOT be used to infer it. The issue remains owned by the same request after
release so a later resume cannot silently create a replacement owner. An existing
child can be looked up by `(parent, child)` through `lookup`, which returns the
retained route and state. Consumers MUST NOT infer an effective model from the
requested pair.

- `reserve(parent, request, issue, model, thinking, limit=8)` atomically admits
  one slot or returns `queued`. An already reserved request returns `existing`;
  an existing child admitted for another turn returns `resumed`. Neither result
  authorizes creating another child. Competing owners for the same issue fail.
  The effective ceiling
  is the smaller of eight and the caller's explicit lower limit. Resumption of a
  released request reserves capacity again before reactivating its old owner.
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
MUST reconcile existing owners before admitting new work. Until the
installed app provides a verified event and correlation route, the store and its
tests do not prove that create, resume, or delivery calls are enforced.

The provisional `supervision_observation.observe_create` adapter consumes a
trusted `PostToolUse` event. It uses `session_id` as the parent key and
`tool_use_id` as the request key, and updates only an already existing matching
reservation. It accepts an unambiguous ready or pending MCP result and keeps
unknown, failed, malformed, or conflicting results reserved. This is not yet
registered as a hook: a caller still needs a trusted way to associate the
pre-call reservation and issue with that call ID, and actual app event and
response delivery remains unverified. The documented event fields and hook
coverage are described in the [Codex hooks reference](https://developers.openai.com/codex/hooks).
