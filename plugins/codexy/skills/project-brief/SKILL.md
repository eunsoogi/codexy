---
name: project-brief
description: Use when a person returns to an ongoing task and needs a read-only brief of recorded current state without changing ownership, status, plans, or actions.
---

# Project Brief

## Trigger

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
- MUST use the literal string `unavailable` for a missing scalar and the single
  item `unavailable` for a missing list.
- MUST NOT write state, direct a child, or change owner, status, next action, or
  done condition.

## Projection

MUST emit the result described in [contract.md](contract.md), with exactly these
keys in this order and no other prose:

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

MUST copy `verified_phase`, `decision_required`, `next_action`, and `done_when`
only when each is recorded as that field. MUST keep merge, publication, public
verification, and milestone closure as distinct phases. A completed proof MUST
NOT become task completion. MUST report a current recorded head change in
`changes_since_touch`; MUST NOT derive a change from stale memory alone.

## Preservation

The projection MUST remain read-only. Repository, GitHub, task, release, and
proof state MUST remain byte-for-byte or state-for-state identical before and
after use.
