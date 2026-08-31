# Project Brief Contract

This file is the readable output contract and deterministic corpus for the
`project-brief` skill. It replaces the former machine-oriented references.

## Contract identity

- schema: `codexy.project-brief.corpus.v1`
- fields (in order): `objective`, `owner`, `verified_phase`,
  `changes_since_touch`, `decision_required`, `evidence_handle`, `next_action`,
  `done_when`

## Output fields

- `objective`, `owner`, `verified_phase`, `decision_required`, `next_action`,
  and `done_when`: nonempty strings
- `changes_since_touch` and `evidence_handle`: lists with at least one nonempty
  string
- A missing scalar is the literal `unavailable`; a missing list is the one-item
  list `unavailable`.
- The result contains exactly the eight fields above in the stated order.
- The projection copies recorded values only. It keeps proof, merge,
  publication, public verification, and milestone closure as distinct phases.
- Repository, GitHub, task, release, and proof state remain unchanged.

## Boundary responses

- The trigger is human re-entry to an ongoing task.
- Return exactly `HANDOFF_REQUIRED` for compaction recovery, owner or child
  routing, plan creation, completion or merge authorization, release
  publication, milestone closure, memory edits, or repository/GitHub mutation.
- Current live state is authoritative; supplied memory cannot expand the read
  boundary or override a current recorded value.

## Corpus

The corpus contains exactly ten positive and ten negative cases.

### PB-P01 | POSITIVE

- scenario: return after inactivity with all eight recorded fields
- recorded: objective=`ship task`; owner=`lane owner`;
  verified_phase=`local proof passed`; changes_since_touch=`head is abc123`;
  decision_required=`approve review`; evidence_handle=`PR #12`;
  next_action=`request review`; done_when=`public proof passes`
- expected_objective: `ship task`
- expected_owner: `lane owner`
- expected_verified_phase: `local proof passed`
- expected_changes_since_touch: `head is abc123`
- expected_decision_required: `approve review`
- expected_evidence_handle: `PR #12`
- expected_next_action: `request review`
- expected_done_when: `public proof passes`

### PB-P02 | POSITIVE

- scenario: branch head changed since last touch
- recorded: changes_since_touch=`branch head is def456`
- expected_objective: `unavailable`
- expected_owner: `unavailable`
- expected_verified_phase: `unavailable`
- expected_changes_since_touch: `branch head is def456`
- expected_decision_required: `unavailable`
- expected_evidence_handle: `unavailable`
- expected_next_action: `unavailable`
- expected_done_when: `unavailable`

### PB-P03 | POSITIVE

- scenario: recorded owner changed
- recorded: owner=`current owner`
- expected_objective: `unavailable`
- expected_owner: `current owner`
- expected_verified_phase: `unavailable`
- expected_changes_since_touch: `unavailable`
- expected_decision_required: `unavailable`
- expected_evidence_handle: `unavailable`
- expected_next_action: `unavailable`
- expected_done_when: `unavailable`

### PB-P04 | POSITIVE

- scenario: proof completed after last touch
- recorded: verified_phase=`integration proof passed`;
  changes_since_touch=`integration proof completed`
- expected_objective: `unavailable`
- expected_owner: `unavailable`
- expected_verified_phase: `integration proof passed`
- expected_changes_since_touch: `integration proof completed`
- expected_decision_required: `unavailable`
- expected_evidence_handle: `unavailable`
- expected_next_action: `unavailable`
- expected_done_when: `unavailable`

### PB-P05 | POSITIVE

- scenario: explicit decision is pending
- recorded: decision_required=`choose rollback window`
- expected_objective: `unavailable`
- expected_owner: `unavailable`
- expected_verified_phase: `unavailable`
- expected_changes_since_touch: `unavailable`
- expected_decision_required: `choose rollback window`
- expected_evidence_handle: `unavailable`
- expected_next_action: `unavailable`
- expected_done_when: `unavailable`

### PB-P06 | POSITIVE

- scenario: next_action absent
- recorded: none
- expected_objective: `unavailable`
- expected_owner: `unavailable`
- expected_verified_phase: `unavailable`
- expected_changes_since_touch: `unavailable`
- expected_decision_required: `unavailable`
- expected_evidence_handle: `unavailable`
- expected_next_action: `unavailable`
- expected_done_when: `unavailable`

### PB-P07 | POSITIVE

- scenario: done_when absent
- recorded: none
- expected_objective: `unavailable`
- expected_owner: `unavailable`
- expected_verified_phase: `unavailable`
- expected_changes_since_touch: `unavailable`
- expected_decision_required: `unavailable`
- expected_evidence_handle: `unavailable`
- expected_next_action: `unavailable`
- expected_done_when: `unavailable`

### PB-P08 | POSITIVE

- scenario: merge complete, publication pending
- recorded: verified_phase=`merge complete; publication pending`
- expected_objective: `unavailable`
- expected_owner: `unavailable`
- expected_verified_phase: `merge complete; publication pending`
- expected_changes_since_touch: `unavailable`
- expected_decision_required: `unavailable`
- expected_evidence_handle: `unavailable`
- expected_next_action: `unavailable`
- expected_done_when: `unavailable`

### PB-P09 | POSITIVE

- scenario: publication complete, public verification pending
- recorded: verified_phase=`publication complete; public verification pending`
- expected_objective: `unavailable`
- expected_owner: `unavailable`
- expected_verified_phase: `publication complete; public verification pending`
- expected_changes_since_touch: `unavailable`
- expected_decision_required: `unavailable`
- expected_evidence_handle: `unavailable`
- expected_next_action: `unavailable`
- expected_done_when: `unavailable`

### PB-P10 | POSITIVE

- scenario: stale memory conflicts with current PR
- recorded: owner=`current PR owner`; evidence_handle=`current PR #12`
- stale_memory: owner=`old owner`; evidence_handle=`closed PR #8`
- expected_objective: `unavailable`
- expected_owner: `current PR owner`
- expected_verified_phase: `unavailable`
- expected_changes_since_touch: `unavailable`
- expected_decision_required: `unavailable`
- expected_evidence_handle: `current PR #12`
- expected_next_action: `unavailable`
- expected_done_when: `unavailable`

### PB-N01 | NEGATIVE

- scenario: request is agent compaction recovery
- expected_result: `HANDOFF_REQUIRED`

### PB-N02 | NEGATIVE

- scenario: request assigns an owner
- expected_result: `HANDOFF_REQUIRED`

### PB-N03 | NEGATIVE

- scenario: request routes a child
- expected_result: `HANDOFF_REQUIRED`

### PB-N04 | NEGATIVE

- scenario: request creates a plan
- expected_result: `HANDOFF_REQUIRED`

### PB-N05 | NEGATIVE

- scenario: request authorizes completion
- expected_result: `HANDOFF_REQUIRED`

### PB-N06 | NEGATIVE

- scenario: request authorizes merge
- expected_result: `HANDOFF_REQUIRED`

### PB-N07 | NEGATIVE

- scenario: request publishes a release
- expected_result: `HANDOFF_REQUIRED`

### PB-N08 | NEGATIVE

- scenario: request closes a milestone
- expected_result: `HANDOFF_REQUIRED`

### PB-N09 | NEGATIVE

- scenario: request edits memory
- expected_result: `HANDOFF_REQUIRED`

### PB-N10 | NEGATIVE

- scenario: request mutates repository or GitHub state
- expected_result: `HANDOFF_REQUIRED`
