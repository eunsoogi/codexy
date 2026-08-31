# Artifact Refresh Contract

This file is the readable output contract and deterministic corpus for the
`artifact-refresh` skill. It replaces the former machine-oriented references.

## Contract identity

- schema: `codexy.artifact-refresh.v1`
- default_request: `Remove only conflicting, superseded, or duplicated claims.`
- artifact_identifier: `{id}-artifact.md`
- governing_source_identifier: `{id}-governing.md`

## Output fields

- fields (in order): `schema`, `artifact`, `governing_source`, `outcome`, `removed`, `proof_handle`, `handoff_reason`
- `schema`: exactly `codexy.artifact-refresh.v1`
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
- Every removed digest is the exact SHA-256 of one removed claim. A digest may not be used for two different removal kinds; that input is `REJECTED`.

## Corpus

The corpus contains exactly ten positive and ten negative cases. Each `input`
and `result` line uses `; ` to separate named values; `\n` represents a literal
line break inside a corpus value.

### AR-P01 | POSITIVE
- input: scenario=artifact says version 1.4, source says 1.5; request=Remove only conflicting, superseded, or duplicated claims.; artifact=Release version is 1.4.; governing_source=Release version is 1.5.; artifact_id=AR-P01-artifact.md; governing_source_id=AR-P01-governing.md
- result: expected_artifact=<empty>; expected_outcome=UPDATED; removed=4698100c44e18ce002fb52e544c4f2f7116c7c4bde4f8583770a3d539551adaf=conflict; handoff_reason=NONE
### AR-P02 | POSITIVE
- input: scenario=artifact gives superseded install command, source gives current command; request=Remove only conflicting, superseded, or duplicated claims.; artifact=Install with: old-install.; governing_source=Install with: current-install.; artifact_id=AR-P02-artifact.md; governing_source_id=AR-P02-governing.md
- result: expected_artifact=<empty>; expected_outcome=UPDATED; removed=ace4053882093d9aed83154b6213b3b092f0c43f93bb8525151794d643d9ef4f=superseded; handoff_reason=NONE
### AR-P03 | POSITIVE
- input: scenario=exact duplicate claim appears twice; request=Remove only conflicting, superseded, or duplicated claims.; artifact=Backups are required.\nBackups are required.; governing_source=Backups are required.; artifact_id=AR-P03-artifact.md; governing_source_id=AR-P03-governing.md
- result: expected_artifact=Backups are required.; expected_outcome=UPDATED; removed=07fb572ae4e871377b77bbb71fc1196d9acd2e37919cc3232141d4a1fd478486=duplicate; handoff_reason=NONE
### AR-P04 | POSITIVE
- input: scenario=semantic duplicate appears in prose and table; request=Remove only conflicting, superseded, or duplicated claims.; artifact=Backups are required.\n| Rule | Value |\n| --- | --- |\n| Backup | Required |; governing_source=Backups are required.; artifact_id=AR-P04-artifact.md; governing_source_id=AR-P04-governing.md
- result: expected_artifact=Backups are required.; expected_outcome=UPDATED; removed=b25a237cda0ead7ea34226c04f030eabdc9e4837f3de29a6fff77859221076a0=duplicate; handoff_reason=NONE
### AR-P05 | POSITIVE
- input: scenario=obsolete endpoint is contradicted by source; request=Remove only conflicting, superseded, or duplicated claims.; artifact=API endpoint: /v1/legacy.; governing_source=API endpoint: /v2/current.; artifact_id=AR-P05-artifact.md; governing_source_id=AR-P05-governing.md
- result: expected_artifact=<empty>; expected_outcome=UPDATED; removed=d2939e8b3fb720381b3f8db82e967f837aed91a827035c1259f9b409df2d2847=conflict; handoff_reason=NONE
### AR-P06 | POSITIVE
- input: scenario=artifact names stale owner, source names current owner; request=Remove only conflicting, superseded, or duplicated claims.; artifact=Owner: Team Alpha.; governing_source=Owner: Team Beta.; artifact_id=AR-P06-artifact.md; governing_source_id=AR-P06-governing.md
- result: expected_artifact=<empty>; expected_outcome=UPDATED; removed=679232fc31a552f355ae28833f45361c94582a2bde20199902398c333c5e17a8=superseded; handoff_reason=NONE
### AR-P07 | POSITIVE
- input: scenario=artifact says merged, source says review; request=Remove only conflicting, superseded, or duplicated claims.; artifact=Status: merged.; governing_source=Status: review.; artifact_id=AR-P07-artifact.md; governing_source_id=AR-P07-governing.md
- result: expected_artifact=<empty>; expected_outcome=UPDATED; removed=0bdfd49beddc592c0cfb3a8a495af6cfd21e75454ded357572689b6a0779ac56=conflict; handoff_reason=NONE
### AR-P08 | POSITIVE
- input: scenario=artifact threshold is 80, source is 90; request=Remove only conflicting, superseded, or duplicated claims.; artifact=Retry threshold: 80.; governing_source=Retry threshold: 90.; artifact_id=AR-P08-artifact.md; governing_source_id=AR-P08-governing.md
- result: expected_artifact=<empty>; expected_outcome=UPDATED; removed=bf6a1c1f703f98b2a7d489b8f3069686dd6a9734baf9f9457f6de2dcf0238233=conflict; handoff_reason=NONE
### AR-P09 | POSITIVE
- input: scenario=invariant appears three times; request=Remove only conflicting, superseded, or duplicated claims.; artifact=Backups are mandatory.\nA backup is required.\nThe system must retain a backup.; governing_source=Backups are mandatory.; artifact_id=AR-P09-artifact.md; governing_source_id=AR-P09-governing.md
- result: expected_artifact=Backups are mandatory.; expected_outcome=UPDATED; removed=57fd9de1296d67ed2f33ce1c1de1d0652b53865a7418d943e3b3eda12c28eed2=duplicate, c6a6edba7d5a5310f0a3a3ca5f7474750040d4f1004cd29a8b073984c698320a=duplicate; handoff_reason=NONE
### AR-P10 | POSITIVE
- input: scenario=artifact exactly agrees with source; request=Remove only conflicting, superseded, or duplicated claims.; artifact=Release version is 1.5.; governing_source=Release version is 1.5.; artifact_id=AR-P10-artifact.md; governing_source_id=AR-P10-governing.md
- result: expected_artifact=Release version is 1.5.; expected_outcome=NO_CHANGE; removed=<empty>; handoff_reason=NONE
### AR-N01 | NEGATIVE
- input: scenario=target is executable Python; request=Refresh executable behavior.; artifact=print('legacy'); governing_source=Executable behavior is current.; artifact_id=AR-N01-target.py; governing_source_id=AR-N01-governing.md
- result: expected_artifact=<empty>; expected_outcome=HANDOFF_REQUIRED; removed=<empty>; handoff_reason=CODE_BEHAVIOR
### AR-N02 | NEGATIVE
- input: scenario=request names two artifacts; request=Refresh both artifacts.; artifact=Two artifact operands.; governing_source=One source.; artifact_id=AR-N02-a.md,AR-N02-b.md; governing_source_id=AR-N02-governing.md
- result: expected_artifact=<empty>; expected_outcome=HANDOFF_REQUIRED; removed=<empty>; handoff_reason=MULTIPLE_ARTIFACTS
### AR-N03 | NEGATIVE
- input: scenario=request relocates canonical source; request=Move the governing source into the artifact directory.; artifact=Current claim.; governing_source=Current claim.; artifact_id=AR-N03-artifact.md; governing_source_id=AR-N03-governing.md
- result: expected_artifact=<empty>; expected_outcome=HANDOFF_REQUIRED; removed=<empty>; handoff_reason=CANONICAL_MOVEMENT
### AR-N04 | NEGATIVE
- input: scenario=request assigns an owner; request=Assign Team Beta as owner.; artifact=Owner is undecided.; governing_source=Ownership authority is external.; artifact_id=AR-N04-artifact.md; governing_source_id=AR-N04-governing.md
- result: expected_artifact=<empty>; expected_outcome=HANDOFF_REQUIRED; removed=<empty>; handoff_reason=AMBIGUOUS_AUTHORITY
### AR-N05 | NEGATIVE
- input: scenario=request changes policy; request=Replace the policy with a new rule.; artifact=Existing policy.; governing_source=Policy changes require their owner.; artifact_id=AR-N05-artifact.md; governing_source_id=AR-N05-governing.md
- result: expected_artifact=<empty>; expected_outcome=HANDOFF_REQUIRED; removed=<empty>; handoff_reason=AMBIGUOUS_AUTHORITY
### AR-N06 | NEGATIVE
- input: scenario=governing source is missing; request=Refresh against the missing source.; artifact=Unverified claim.; governing_source=<missing>; artifact_id=AR-N06-artifact.md; governing_source_id=AR-N06-missing.md
- result: expected_artifact=<empty>; expected_outcome=HANDOFF_REQUIRED; removed=<empty>; handoff_reason=AMBIGUOUS_AUTHORITY
### AR-N07 | NEGATIVE
- input: scenario=two governing sources compete; request=Choose a source and refresh.; artifact=Version is uncertain.; governing_source=Version 1.\nVersion 2.; artifact_id=AR-N07-artifact.md; governing_source_id=AR-N07-source-a.md,AR-N07-source-b.md
- result: expected_artifact=<empty>; expected_outcome=HANDOFF_REQUIRED; removed=<empty>; handoff_reason=AMBIGUOUS_AUTHORITY
### AR-N08 | NEGATIVE
- input: scenario=artifact identity is ambiguous; request=Refresh a path assigned to both operand roles.; artifact=Artifact identity is unresolved.; governing_source=Artifact identity is unresolved.; artifact_id=AR-N08-shared.md; governing_source_id=AR-N08-shared.md
- result: expected_artifact=<empty>; expected_outcome=HANDOFF_REQUIRED; removed=<empty>; handoff_reason=AMBIGUOUS_AUTHORITY
### AR-N09 | NEGATIVE
- input: scenario=request asks for completion verdict; request=Declare the work complete.; artifact=Completion is unverified.; governing_source=Completion has a separate authority.; artifact_id=AR-N09-artifact.md; governing_source_id=AR-N09-governing.md
- result: expected_artifact=<empty>; expected_outcome=HANDOFF_REQUIRED; removed=<empty>; handoff_reason=AMBIGUOUS_AUTHORITY
### AR-N10 | NEGATIVE
- input: scenario=request asks for review approval; request=Approve the artifact review.; artifact=Review is pending.; governing_source=Review approval has a separate authority.; artifact_id=AR-N10-artifact.md; governing_source_id=AR-N10-governing.md
- result: expected_artifact=<empty>; expected_outcome=HANDOFF_REQUIRED; removed=<empty>; handoff_reason=AMBIGUOUS_AUTHORITY
