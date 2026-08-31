# Blind Read Contract

This file is the readable output contract and deterministic corpus for the
`blind-read` skill. It replaces the former machine-oriented references.

## Contract identity

- fields (in order): `interpreted_purpose`, `unresolved_reference`,
  `action_blocker`

## Output fields

- `interpreted_purpose`: a nonempty string
- `unresolved_reference`: a unique list of nonempty strings in first-appearance
  order
- `action_blocker`: a unique list of nonempty strings in first-appearance order
- The result contains exactly the three fields above and no prose or code fence.
- Empty lists mean only that no blocking gap was found.

## Boundary responses

- Inputs MUST contain exactly `artifact_bytes`, `intended_audience`, and
  `intended_action`; missing input MUST return `BLOCKED_INPUT` without analysis.
- Requests outside the trigger MUST return exactly one
  `HANDOFF_REQUIRED:<reason>` string before artifact analysis.
- The first matching reason MUST be selected in this order: `FACT_CHECK`,
  `CORRECTNESS_VERDICT`, `MUTATION`, `REFRESH`, `AUTHOR_INTERVIEW`,
  `HIDDEN_CRITERIA`, `HISTORY`, `VOTE`, `STATUS_RECONSTRUCTION`,
  `MULTIPLE_ARTIFACTS`.
- A fresh reader MUST use only the supplied artifact bytes and MUST NOT edit,
  normalize, fact-check, judge, reconstruct history, or compare artifacts.

## Corpus

The corpus contains exactly ten positive and ten negative cases.

### BR-P01 | POSITIVE

- artifact_bytes: `Run make init.`
- intended_audience: `new contributor`
- intended_action: `initialize workspace`
- expected_purpose: `initialize workspace`
- expected_unresolved_reference: `<empty>`
- expected_action_blocker: `<empty>`

### BR-P02 | POSITIVE

- artifact_bytes: `Reproduce with log from <run>.`
- intended_audience: `maintainer`
- intended_action: `reproduce issue`
- expected_purpose: `reproduce issue`
- expected_unresolved_reference: `<run>`
- expected_action_blocker: `run identifier is missing`

### BR-P03 | POSITIVE

- artifact_bytes: `Owner release; continue at job 44.`
- intended_audience: `handoff owner`
- intended_action: `resume work`
- expected_purpose: `resume release job 44`
- expected_unresolved_reference: `<empty>`
- expected_action_blocker: `<empty>`

### BR-P04 | POSITIVE

- artifact_bytes: `Export TOKEN, then run deploy.`
- intended_audience: `operator`
- intended_action: `deploy service`
- expected_purpose: `deploy service`
- expected_unresolved_reference: `TOKEN`
- expected_action_blocker: `TOKEN source is missing`

### BR-P05 | POSITIVE

- artifact_bytes: `POST /v1/items with ItemRequest.`
- intended_audience: `API client author`
- intended_action: `create item`
- expected_purpose: `create item through POST /v1/items`
- expected_unresolved_reference: `ItemRequest`
- expected_action_blocker: `ItemRequest schema is missing`

### BR-P06 | POSITIVE

- artifact_bytes: `Set mode=fast in app.toml.`
- intended_audience: `administrator`
- intended_action: `enable fast mode`
- expected_purpose: `enable fast mode in app.toml`
- expected_unresolved_reference: `<empty>`
- expected_action_blocker: `<empty>`

### BR-P07 | POSITIVE

- artifact_bytes: `Version 2 removes old login; migrate with guide G.`
- intended_audience: `upgrader`
- intended_action: `upgrade login`
- expected_purpose: `upgrade removed login flow`
- expected_unresolved_reference: `guide G`
- expected_action_blocker: `migration guide G is unresolved`

### BR-P08 | POSITIVE

- artifact_bytes: `If E42 occurs, run repair <cmd>.`
- intended_audience: `support engineer`
- intended_action: `repair E42`
- expected_purpose: `repair E42`
- expected_unresolved_reference: `<cmd>`
- expected_action_blocker: `repair command is missing`

### BR-P09 | POSITIVE

- artifact_bytes: `Gateway calls Worker as defined in diagram D.`
- intended_audience: `architect`
- intended_action: `trace request`
- expected_purpose: `trace Gateway to Worker request`
- expected_unresolved_reference: `diagram D`
- expected_action_blocker: `diagram D is unavailable`

### BR-P10 | POSITIVE

- artifact_bytes: `Run verify.sh; success is exit 0.`
- intended_audience: `release operator`
- intended_action: `verify release`
- expected_purpose: `verify release with verify.sh`
- expected_unresolved_reference: `<empty>`
- expected_action_blocker: `<empty>`

### BR-N01 | NEGATIVE

- artifact_bytes: `Claim X`
- intended_audience: `reader`
- intended_action: `fact-check X`
- expected_result: `HANDOFF_REQUIRED:FACT_CHECK`

### BR-N02 | NEGATIVE

- artifact_bytes: `Patch`
- intended_audience: `reviewer`
- intended_action: `approve code correctness`
- expected_result: `HANDOFF_REQUIRED:CORRECTNESS_VERDICT`

### BR-N03 | NEGATIVE

- artifact_bytes: `Draft`
- intended_audience: `editor`
- intended_action: `rewrite artifact`
- expected_result: `HANDOFF_REQUIRED:MUTATION`

### BR-N04 | NEGATIVE

- artifact_bytes: `Version 1`
- intended_audience: `maintainer`
- intended_action: `refresh against source`
- expected_result: `HANDOFF_REQUIRED:REFRESH`

### BR-N05 | NEGATIVE

- artifact_bytes: `Decision`
- intended_audience: `author`
- intended_action: `explain what you meant`
- expected_result: `HANDOFF_REQUIRED:AUTHOR_INTERVIEW`

### BR-N06 | NEGATIVE

- artifact_bytes: `Submission`
- intended_audience: `judge`
- intended_action: `score against hidden rubric`
- expected_result: `HANDOFF_REQUIRED:HIDDEN_CRITERIA`

### BR-N07 | NEGATIVE

- artifact_bytes: `Summary`
- intended_audience: `reader`
- intended_action: `explain session history`
- expected_result: `HANDOFF_REQUIRED:HISTORY`

### BR-N08 | NEGATIVE

- artifact_bytes: `Options A/B`
- intended_audience: `team`
- intended_action: `vote for A or B`
- expected_result: `HANDOFF_REQUIRED:VOTE`

### BR-N09 | NEGATIVE

- artifact_bytes: `Task`
- intended_audience: `manager`
- intended_action: `reconstruct live status`
- expected_result: `HANDOFF_REQUIRED:STATUS_RECONSTRUCTION`

### BR-N10 | NEGATIVE

- artifact_bytes: `Artifact A and Artifact B`
- intended_audience: `reader`
- intended_action: `compare both artifacts`
- expected_result: `HANDOFF_REQUIRED:MULTIPLE_ARTIFACTS`
