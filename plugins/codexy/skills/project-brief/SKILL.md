---
name: project-brief
description: Present a read-only eight-field human re-entry brief from recorded current task, Git or PR, release, and proof state without inventing or changing project state.
---

# Project Brief

## Use when

Use only when a person returns to an ongoing task and needs a one-screen view
of its recorded current state.

MUST return `HANDOFF_REQUIRED` when the request is agent compaction recovery,
assigns an owner, routes a child, creates a plan, authorizes completion or
merge, publishes a release, closes a milestone, edits memory, or mutates the
repository or GitHub. Those requests belong to their existing authorities.

## Read boundary

MUST read only the named current task, Git or PR, release, and proof state
needed for the eight output fields. MUST NOT write state, direct a child, alter
owner or status, or create a next action or done condition.

Current live state MUST outrank stale memory. MUST copy only recorded values.
For a missing scalar, use the literal string `unavailable`; for a missing list,
use the single-item list `["unavailable"]`. MUST NOT infer across phase
boundaries: proof, task completion, merge, publication, public verification,
and milestone closure remain distinct.

## Output

Return exactly one object conforming to `schema.json`, in this field order:

1. `objective`
2. `owner`
3. `verified_phase`
4. `changes_since_touch`
5. `decision_required`
6. `evidence_handle`
7. `next_action`
8. `done_when`

MUST NOT add commentary or fields. `changes_since_touch` contains only recorded
changes, and `evidence_handle` contains only current references. If current
state conflicts with stale memory, use current state without rewriting memory.

Before returning, confirm the projection has exactly eight fields and that the
operation made no repository, GitHub, task, release, or proof mutation. If that
cannot be confirmed, return `HANDOFF_REQUIRED` instead of a brief.
