# Review-Response Cluster Receipts

## Required Procedure

1. [receipt-create] Before editing actionable review feedback, MUST create one typed JSON receipt.
2. [receipt-validate] Before implementation, MUST validate that exact receipt file with `scripts/validate-plugin-config --check-review-response-cluster --review-response-cluster-file receipt.json`.

3. [case-exception-prohibition] During repair, MUST NOT accept a case-specific exception as structural evidence.
4. [reopen-evidence-restriction] Non-reopened receipt states MUST NOT include `reopen` evidence.
5. [final-receipt-validate] After addressing feedback and before push or handoff, MUST set the receipt state to `repaired` or `reopened` and validate that exact final-state file with `scripts/validate-plugin-config --check-review-response-cluster --review-response-cluster-file receipt.json`.

## Typed Receipt

The file MUST be a typed `ReviewClusterReceipt` with a `state` of `planned`,
`repaired`, or `reopened`, plus a nonempty `clusters` array. Each cluster MUST
contain non-blank `defect_class`, `violated_invariant`, `structural_boundary`,
`threads`, `matrix.positive`, and `matrix.negative` values. Defect classes are
canonicalized before equality checks, so whitespace and case variants MUST NOT
split one defect class.

Validation is phase-ordered: every supplied scalar, list, repair, and reopen
subobject is parsed, normalized, and validated before state rules are applied.
`planned` clusters may omit `repair`, but a supplied repair MUST be structural;
`repaired` clusters MUST include a structural repair; `reopened` clusters MUST
include a structural repair and every `reopened` receipt MUST include `reopen`
evidence for its reopened class. Non-reopened clusters MUST NOT include `reopen`
evidence. Canonical identity uses Unicode
normalization, full case folding, and Unicode punctuation/separator removal,
while retaining materially different alphanumeric identifier content.
Default-ignorable code points, controls, and format-only content MUST NOT
contribute identity or satisfy required evidence. A normalized identity MUST
contain visible material content; combining marks are valid after a material
base character but MUST NOT satisfy that requirement alone.
The positive and negative matrix arrays MUST each be nonempty, canonically
unique, and canonically disjoint. Their source strings remain representative
display evidence; validation compares their semantic identities instead.
Canonical identity re-normalizes the filtered material stream, so it is a
fixed point even when removed content had separated a base and combining mark.

For `repaired` and `reopened` states, every cluster MUST have:

```json
{
  "repair": {
    "kind": "structural",
    "boundary": "parser or normalization boundary",
    "strategy": "one structural repair",
    "removed_case_specific_behavior": true
  }
}
```

`case_exception` repairs MUST NOT be accepted. A reopened cluster MUST add either
`{"kind":"distinct_invariant","invariant":"..."}` or
`{"kind":"structural_repair_incomplete","evidence":"..."}` under
`reopen`.
