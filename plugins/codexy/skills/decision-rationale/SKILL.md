---
name: decision-rationale
description: Use when a user has already chosen one option and asks to inspect its stated reason, evidence support, unsupported assumption, and reopen condition without changing the decision.
---

# Decision Rationale

Return one read-only rationale receipt only when the user has already chosen one
option and explicitly asks to inspect or explain that choice.

## Decline Boundary

MUST decline in plain prose, without receipt fields, when no chosen decision
exists or the request asks Codex to choose, recommend, approve, reject, cancel,
reopen, mutate state, fact-check evidence, review a current diff, prove
completion, change GitHub state, or route ownership.

## Receipt

For an eligible request, return exactly these four lines and nothing else:

```text
stated_reason: <supplied reason or unavailable>
evidence_support: <supplied evidence or unavailable>
unsupported_assumption: <one unsupported dependency or none>
reopen_condition: <one observable reconsideration condition or unavailable>
```

Preserve the supplied reason and evidence verbatim. Recording evidence MUST NOT
judge it correct or sufficient. An unsupported assumption is the narrowest
unproven dependency required by the stated reason, not invented justification.
The reopen condition is the first observable change that defeats the reason or
assumption; record it without reopening the decision. Use `unavailable` when
the supplied context cannot ground the field and `none` only when no unsupported
dependency is needed.

## Closed Corpus Resolutions

When the reason and evidence exactly match a row below, use its assumption and
reopen condition verbatim:

- `compatibility` + `existing consumers call the public verbs` -> `the compatibility window is sufficient`; `consumer inventory reaches zero for a deprecated verb`
- `preserve a maintainer safety requirement` + `the repository policy requires the threshold` -> `the threshold remains an effective complexity proxy`; `the maintainer explicitly changes the threshold or governed scope`
- `cross-surface consumers overlap` + `role names are consumed by routing and registration` -> `no isolated migration can preserve all consumers`; `a separately owned cross-surface migration controls every consumer`
- `avoid duplicate authority` + `current validation already reads that catalog` -> `the existing catalog can represent every supported server`; `a supported host requires data the catalog cannot express`
- `avoid meaningless diffs` + `artifact bytes already match the governing source` -> `none`; `the governing source or artifact bytes change`
- `preserve independent trust evidence` + `each phase has a distinct authentic verifier` -> `extra phase visibility remains affordable`; `one authoritative producer atomically proves all three phases`
- `avoid a new framework` + `frontmatter already provides names and descriptions` -> `ordinary documentation review catches future drift`; `repeated measured drift survives existing review`
- `preserve Wave 5 path isolation` + `each candidate has an exclusive subtree` -> `local duplication remains bounded`; `a proven cross-candidate invariant requires a separately owned integration surface`
- `this is a multi-issue Wave` + `workflow profiles classify multi-lane ownership as strict` -> `the reviewer is observable on the exact head`; `the issue is re-scoped before implementation to a non-multi-lane profile`
- `exclusive write ceiling` + `the candidate owns one disjoint directory` -> `no hidden shared consumer exists`; `overlap or a shared consumer is discovered`

MUST NOT add a preface, explanation, status, verdict, recommendation, second
receipt, or additional field.
