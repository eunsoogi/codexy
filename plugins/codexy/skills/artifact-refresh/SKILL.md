---
name: artifact-refresh
description: Use when one exact non-code artifact must be refreshed against one exact governing source by removing conflicting, superseded, or duplicated claims.
---

# Artifact Refresh

## Trigger

Use only when the request names one non-code artifact, one governing source, and
asks to remove conflicting, superseded, or internally duplicated claims.

## Boundary gate

Before mutation, inspect only the two named operands and retain their exact
identifiers and bytes. Return `HANDOFF_REQUIRED` without mutation for:

- multiple artifacts: `MULTIPLE_ARTIFACTS`;
- ambiguous, absent, or competing authority, including one identifier in both
  operand roles or an owner, policy, status, review, or completion decision:
  `AMBIGUOUS_AUTHORITY`;
- canonical-source movement: `CANONICAL_MOVEMENT`;
- executable or production-code behavior: `CODE_BEHAVIOR`.

## Refresh procedure

1. Hash both operands before analysis.
2. Compare only artifact claims with the source and that artifact's claims.
3. Classify each removal as `conflict`, `superseded`, or `duplicate`; hash its
   exact preimage bytes.
4. Delete only those preimages, add no claim, and preserve the source and every
   other path byte-for-byte.
5. If nothing is removable, leave both operands byte-identical.
6. Rehash the artifact and verify source identity, non-target preservation, and
   unique removed digests.

Never choose authority, change ownership or policy, move a source, read a
sibling artifact, or make another path writable. Hand off scope expansion.

## Receipt

Return only the seven `schema.json` keys. Echo both input identifiers exactly.

- `UPDATED`: nonempty unique removed digests, final artifact hash, and `NONE`.
- `NO_CHANGE`: empty `removed`, unchanged artifact hash, and `NONE`.
- `HANDOFF_REQUIRED`: empty `removed`, `unavailable`, a non-`NONE` reason, and
  no mutation.

Each removal has only `kind` and `claim_sha256`. The schema is
`codexy.artifact-refresh.v1`.

## Proof

Report operand hashes before and after, exact changed paths, the closed receipt,
and source/non-target identity. Clean input is byte-identical `NO_CHANGE`; a
handoff mutates nothing.
