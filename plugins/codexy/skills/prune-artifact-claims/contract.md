# Prune Artifact Claims Contract

This file is the readable output contract for the `prune-artifact-claims` skill.
It replaces the former machine-oriented references.

## Contract identity

- schema: `codexy.prune-artifact-claims.v1`
- default_request: `Remove only conflicting, superseded, or duplicated claims.`
- artifact_identifier: `{id}-artifact.md`
- governing_source_identifier: `{id}-governing.md`

## Output fields

- fields (in order): `schema`, `artifact`, `governing_source`, `outcome`,
  `removed`, `proof_handle`, `handoff_reason`
- `schema`: exactly `codexy.prune-artifact-claims.v1`
- `artifact`: a nonempty string containing the exact input identifier
- `governing_source`: a nonempty string containing the exact input identifier
- `outcome`: one of `UPDATED`, `NO_CHANGE`, or `HANDOFF_REQUIRED`
- `removed`: a map keyed by exactly 64 lowercase hexadecimal characters, whose
  values are `conflict`, `superseded`, or `duplicate`
- `proof_handle`: `unavailable` or `sha256:` followed by exactly 64 lowercase
  hexadecimal characters
- `handoff_reason`: `NONE`, `MULTIPLE_ARTIFACTS`, `AMBIGUOUS_AUTHORITY`,
  `CANONICAL_MOVEMENT`, or `CODE_BEHAVIOR`

## Outcome constraints

- `UPDATED` requires at least one removed entry, a `sha256:` final artifact
  digest, and `handoff_reason` `NONE`.
- `NO_CHANGE` requires no removed entries, a `sha256:` unchanged artifact
  digest, and `handoff_reason` `NONE`.
- `HANDOFF_REQUIRED` requires no removed entries, `proof_handle` `unavailable`,
  and one of the four non-`NONE` handoff reasons.
- Every removed digest is the exact SHA-256 of one removed claim. A digest may
  not be used for two different removal kinds; that input is `REJECTED`.
