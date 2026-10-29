# Blind Read Contract

This file is the readable output contract for the `blind-read` skill. It
replaces the former machine-oriented references.

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
