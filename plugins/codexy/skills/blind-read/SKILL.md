---
name: blind-read
description: Use when a fresh reader must interpret one artifact for one named audience and action without judging, editing, or reconstructing outside context.
---

# Blind Read

## Purpose

Interpret only the supplied artifact bytes from the perspective of the supplied
audience and action. This skill measures whether the reader can understand and
act; it does not certify truth, correctness, quality, completion, or approval.

## Input gate

Inputs MUST contain exactly `artifact_bytes`, `intended_audience`, and
`intended_action`. Each MUST describe one artifact, one audience, and one
action. Missing input MUST return task-level `BLOCKED_INPUT` without analyzing
the artifact.

Before artifact analysis, requests outside this trigger MUST return exactly one
`HANDOFF_REQUIRED:<reason>` string. Select the first matching reason below:

1. `FACT_CHECK` for checking a claim or fact.
2. `CORRECTNESS_VERDICT` for correctness, completion, quality, review, or approval.
3. `MUTATION` for editing, rewriting, or otherwise changing the artifact.
4. `REFRESH` for updating the artifact against another source.
5. `AUTHOR_INTERVIEW` for recovering what the author meant.
6. `HIDDEN_CRITERIA` for judging against criteria not in the artifact.
7. `HISTORY` for using author, session, or task history.
8. `VOTE` for choosing or voting between options.
9. `STATUS_RECONSTRUCTION` for reconstructing live status.
10. `MULTIPLE_ARTIFACTS` for comparing or interpreting more than one artifact.

## Fresh-reader procedure

1. Treat `artifact_bytes` as the complete and only artifact. MUST NOT seek or
   use author intent, history, outside facts, hidden criteria, or another file.
2. Preserve `artifact_bytes` byte-identically. MUST NOT edit, normalize, or
   propose repaired text.
3. State the literal immediate purpose for the named audience and action. Add
   only artifact-explicit details needed to identify the target, endpoint,
   operation, job, error, file, or flow.
4. Record a reference only when the named audience needs its missing value,
   source, definition, schema, command, guide, or diagram to perform the action.
   A complete identifier, filename, command, endpoint, setting, or job number
   is not unresolved merely because it is named.
5. Record the corresponding action blocker as the smallest literal description
   of what is missing, unresolved, or unavailable. Preserve first appearance
   order and remove duplicates.

## Output contract

For an in-trigger request, return only one JSON object matching
[schema.json](schema.json), with no prose or code fence. It MUST contain exactly
`interpreted_purpose`, `unresolved_reference`, and `action_blocker` in that
order. The purpose MUST be a nonempty string; both arrays MUST contain unique
nonempty strings in first appearance order. Empty arrays mean only that no
blocking gap was found.
