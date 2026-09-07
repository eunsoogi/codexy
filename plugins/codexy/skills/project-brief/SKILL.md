---
name: project-brief
description: Use when a person returns to an ongoing task and needs a read-only brief of recorded current state without changing ownership, status, plans, or actions.
---

# Project Brief

## Trigger

A natural-language request for project status MUST receive a concise human
explanation by default. An explicit request for a machine receipt, or an actual
existing machine consumer, selects the machine contract in
[contract.md](contract.md).

MUST use only for human re-entry to an ongoing task. MUST return
`HANDOFF_REQUIRED` when the request concerns agent compaction recovery, assigns
or routes an owner or child, creates a plan, authorizes completion or merge,
publishes a release, closes a milestone, edits memory, or mutates repository or
GitHub state.

## Read boundary

- MUST read only explicitly named current task, Git/PR, proof, and release
  state.
- MUST treat current live state as authoritative over stale memory.
- MUST treat supplied memory only as non-authoritative conflict context; it MUST
  NOT expand the read boundary.
- MUST copy recorded values without inferring status, phase, ownership,
  approval, actions, completion, or missing facts.
- In human mode, MUST describe an unrecorded or unknown fact in the user's
  language without demanding fixed fields, hashes, or parser success.
- In machine mode, MUST use the literal string `unavailable` for a missing
  scalar and the single item `unavailable` for a missing list.
- MUST NOT write state, direct a child, or change owner, status, next action, or
  done condition.

## Human summary (default)

- MUST state the current recorded result, remaining work, blocker reason, and
  next observation or action in the user's language when each is recorded.
- MUST preserve the same facts and uncertainty as the named current state. An
  unknown status MUST remain unknown and MUST NOT become a completion judgment.
- MUST NOT require a machine receipt for an ordinary human status request or for
  understanding or continuing the work.
- MUST NOT add causes, an ETA, or a completion judgment absent from the
  material, and MUST keep proof, merge, publication, public verification, and
  milestone closure distinct.

## Machine receipt

When explicitly requested or invoked by an existing machine consumer, MUST emit
the result described in [contract.md](contract.md), with exactly these keys in
this order and no other prose:

```json
{
  "objective": "recorded or unavailable",
  "owner": "recorded or unavailable",
  "verified_phase": "recorded or unavailable",
  "changes_since_touch": ["recorded change or unavailable"],
  "decision_required": "recorded or unavailable",
  "evidence_handle": ["current reference or unavailable"],
  "next_action": "recorded or unavailable",
  "done_when": "recorded or unavailable"
}
```

In machine mode, MUST copy `verified_phase`, `decision_required`, `next_action`,
and `done_when` only when each is recorded as that field. MUST report a current
recorded head change in `changes_since_touch`; MUST NOT derive a change from
stale memory alone. A completed proof MUST NOT become task completion.

## Preservation

Both modes MUST remain read-only. Repository, GitHub, task, release, and proof
state MUST remain byte-for-byte or state-for-state identical before and after
use.
