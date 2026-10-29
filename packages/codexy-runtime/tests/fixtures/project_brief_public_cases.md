## Corpus

The corpus has ten positive and ten negative cases; PB-P01-P10 use machine mode.
Human mode renders recorded facts verbatim and omits unrecorded fields.

### PB-P01 | POSITIVE

- scenario: return after inactivity with all eight recorded fields
- mode: machine
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
- mode: machine
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
- mode: machine
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
- mode: machine
- recorded: result=`pass`; uncertainty=`none`;
  verified_phase=`integration proof passed`;
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
- mode: machine
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
- mode: machine
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
- mode: machine
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
- mode: machine
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
- mode: machine
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
- mode: machine
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
