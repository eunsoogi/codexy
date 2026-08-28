---
name: artifact-refresh
description: Refresh one exact non-code artifact against one exact governing source by removing only conflicting, superseded, or duplicated claims, or return byte-identical NO_CHANGE or a typed handoff.
---

# Artifact Refresh

## Trigger

Use only when the request identifies exactly one non-code artifact, exactly one governing source, and asks to remove conflicting, superseded, or internally duplicated claims.

## Boundary gate

Before mutation, MUST inspect only the two named operands and MUST retain their exact identifiers and bytes.
MUST return `HANDOFF_REQUIRED` without mutation when any boundary below applies:

- more than one artifact: `MULTIPLE_ARTIFACTS`;
- ambiguous, absent, or competing governing authority, or an owner, policy, status, review, or completion decision: `AMBIGUOUS_AUTHORITY`;
- the same identifier assigned to both operand roles: `AMBIGUOUS_AUTHORITY`;
- movement of the canonical source: `CANONICAL_MOVEMENT`;
- executable or production-code behavior: `CODE_BEHAVIOR`.

## Refresh procedure

1. Hash the artifact and governing source bytes before analysis.
2. Compare only exact artifact claims with the governing source and with other claims in that artifact.
3. Classify each removable claim as `conflict`, `superseded`, or `duplicate` and hash its exact preimage bytes.
4. If removal is required, MUST delete only those exact claim preimages, MUST add no claim, and MUST leave the governing source and every other path byte-identical.
5. If no removable claim exists, MUST leave both operands byte-identical.
6. Rehash the final artifact and verify source-byte identity, non-target preservation, and unique removed digests before returning the receipt.

The skill MUST NOT choose authority, change ownership or policy, move a canonical source, read a sibling artifact, or make a second path writable. Scope expansion requires a typed handoff, not a partial edit.

## Receipt

Return exactly the seven keys defined by `schema.json`: `schema`, `artifact`, `governing_source`, `outcome`, `removed`, `proof_handle`, and `handoff_reason`; no free-form key is allowed. `artifact` and `governing_source` MUST exactly echo the input identifiers.

- `UPDATED` requires one or more removed entries with unique lowercase SHA-256 claim digests, a `sha256:<final-artifact-digest>` proof handle, and `NONE`.
- `NO_CHANGE` requires an empty `removed`, a `sha256:<unchanged-artifact-digest>` proof handle, and `NONE`.
- `HANDOFF_REQUIRED` requires an empty `removed`, `unavailable`, one non-`NONE` handoff reason, and no mutation.

Each removed entry MUST contain only `kind` and `claim_sha256`. The receipt schema is `codexy.artifact-refresh.v1`.

## Proof

MUST report operand hashes before and after, the exact changed-path set, the closed receipt, and whether the source and non-target paths stayed byte-identical. Clean input is valid work and MUST remain byte-identical `NO_CHANGE`; a handoff MUST mutate nothing.
