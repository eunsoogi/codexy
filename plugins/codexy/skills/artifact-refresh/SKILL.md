---
name: artifact-refresh
description: Use when one exact non-code artifact must be refreshed against one exact governing source by removing conflicting, superseded, or duplicated claims.
---

# Artifact Refresh

## Trigger

MUST use only when the request names one non-code artifact, one governing
source, and asks to remove conflicting, superseded, or internally duplicated
claims.

## Boundary gate

Before mutation, MUST inspect only the two named operands and retain their exact
identifiers and bytes. MUST return `HANDOFF_REQUIRED` without mutation for:

- multiple artifacts: `MULTIPLE_ARTIFACTS`;
- ambiguous, absent, or competing authority, including one identifier in both
  operand roles or an owner, policy, status, review, or completion decision:
  `AMBIGUOUS_AUTHORITY`;
- canonical-source movement: `CANONICAL_MOVEMENT`;
- executable or production-code behavior: `CODE_BEHAVIOR`.

## Refresh procedure

1. MUST hash both operands before analysis.
2. MUST compare only artifact claims with the source and that artifact's claims.
3. MUST classify each removal as `conflict`, `superseded`, or `duplicate`; hash
   its exact preimage bytes.
4. MUST delete only those preimages, add no claim, and preserve the source and
   every other path byte-for-byte.
5. If nothing is removable, MUST leave both operands byte-identical.
6. MUST rehash the artifact and verify source identity, non-target preservation,
   and unique removed digests.

MUST NOT choose authority, change ownership or policy, move a source, read a
sibling artifact, or make another path writable. MUST hand off scope expansion.

## Contract

The exact receipt fields and deterministic corpus are maintained in
[contract.md](contract.md).

## Receipt

MUST return only the seven fields listed in [contract.md](contract.md) and echo
both input identifiers exactly.

- `UPDATED` MUST contain a nonempty digest-keyed `removed` map, final artifact
  hash, and `NONE`.
- `NO_CHANGE` MUST contain empty `removed`, the unchanged artifact hash, and
  `NONE`.
- `HANDOFF_REQUIRED` MUST contain empty `removed`, `unavailable`, a non-`NONE`
  reason, and no mutation.

Each removal MUST use its `claim_sha256` as the `removed` map key and its `kind`
as the value. The contract schema MUST be `codexy.artifact-refresh.v1`.

## Proof

MUST report operand hashes before and after, exact changed paths, the closed
receipt, and source/non-target identity. Clean input MUST be byte-identical
`NO_CHANGE`; a handoff MUST mutate nothing.
