# Project Brief Contract

This file is the readable output contract for the `project-brief` skill. Its v1
schema describes the explicit machine receipt; the default human response is a
natural-language summary.

## Contract identity

- schema: `codexy.project-brief.corpus.v1`
- fields (in order): `objective`, `owner`, `verified_phase`,
  `changes_since_touch`, `decision_required`, `evidence_handle`, `next_action`,
  `done_when`

## Output fields

These fields apply to machine receipt mode only.

- `objective`, `owner`, `verified_phase`, `decision_required`, `next_action`,
  and `done_when`: nonempty strings
- `changes_since_touch` and `evidence_handle`: lists with at least one nonempty
  string
- A missing scalar is the literal `unavailable`; a missing list is the one-item
  list `unavailable`.
- The result contains exactly the eight fields above in the stated order.
- The projection copies recorded values only. It keeps proof, merge,
  publication, public verification, and milestone closure as distinct phases.
- `verified_phase` is one opaque nonempty string copied exactly as recorded.
  Separately recorded `result` or `uncertainty` values are not v1 output fields
  and MUST NOT be folded into, split from, or used to rewrite `verified_phase`.

## Mode selection

- A natural-language status request MUST select a concise human summary by
  default.
- An explicit machine/receipt request or an existing machine consumer MUST
  select the eight fields above, in the stated order, without a prose wrapper.
- Both modes MUST use the same current recorded facts and uncertainty. Human
  mode MUST render separately recorded `result` and `uncertainty` values
  verbatim; machine mode MUST copy `verified_phase` exactly as recorded. Neither
  mode changes repository, GitHub, task, release, or proof state.

## Boundary responses

- The trigger is human re-entry to an ongoing task.
- Return exactly `HANDOFF_REQUIRED` for compaction recovery, owner or child
  routing, plan creation, completion or merge authorization, release
  publication, milestone closure, memory edits, or repository/GitHub mutation.
- Current live state is authoritative; supplied memory cannot expand the read
  boundary or override a current recorded value.
