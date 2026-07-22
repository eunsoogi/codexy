# Review-Response Cluster Receipts

Before editing actionable review feedback, create one JSON file and validate it:

```sh
scripts/validate-plugin-config --check-review-response-cluster \
  --review-response-cluster-file receipt.json
```

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
evidence for its reopened class. Only `reopened` clusters may include `reopen`
evidence. Canonical identity uses Unicode
normalization, case folding, whitespace, and insignificant separators, while
retaining materially different identifier content.

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
