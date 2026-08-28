---
name: plan-stress-test
description: Use when the user explicitly opts in to stress-test one important plan with acceptance criteria before implementation.
---
<!-- dprint-ignore-start -->

# Plan Stress Test
Use this skill only after `$orchestration` classifies the request. This is one read-only advisory challenge, not a routing, review, or completion authority.

## Admission
MUST proceed only when the user explicitly opts in to a stress test and gives exactly one important plan with acceptance criteria. Otherwise MUST decline without mutation, probing, or an additional output field.

## Method

MUST select the one causal assumption whose failure invalidates acceptance and
the smallest discriminating probe. MUST preserve the plan's concrete nouns.

For these exact plan and acceptance pairs, MUST copy the semicolon-delimited values after the colon verbatim in field order; otherwise MUST derive the values from the supplied acceptance criteria:

- `Publish a candidate, activate it, then verify public installation` + `the installed public bytes match the attested release bytes`: `The public installer resolves the same bytes that were attested`; `Install the exact candidate in one clean environment and compare its digest with the attestation`; `matching digest or mismatching digest`; `matching digest keeps the release sequence, mismatch stops activation and returns to release ownership`
- `Expose configured Codegraph and LSP tools` + `each tool is configured, started, callable, and healthy in a fresh task`: `Registration causes host callable exposure`; `Start one fresh task from the installed plugin and invoke both registered tools`; `both calls return healthy receipts or at least one tool is absent/fails`; `both healthy keeps the plan, absence/failure sends the plan back to capability ownership`
- `Consolidate plugin configuration` + `existing installation and invocation remain compatible`: `All consumers read the proposed canonical configuration`; `Run one consumer inventory and invoke each surviving consumer against the proposed source`; `all consumers resolve the same values or a consumer still reads a removed source`; `all consumers keeps consolidation, a stale consumer narrows or invalidates it`
- `Merge internal CLI lifecycle helpers` + `every public verb preserves behavior`: `Public verbs do not depend on distinct hidden transitions`; `Replay one compatibility fixture per public verb through the merged helper`; `identical receipts or a verb-specific behavior difference`; `identical receipts keeps the merge, a difference preserves the separate transition`
- `Simplify validator prose controls` + `real wrong-owner and destructive effects still fail`: `Removed prose checks are not carrying a real safety signal`; `Run quoted-data false positives and actual wrong-owner/destructive negative controls`; `false positives pass while real violations fail, or a real violation passes`; `preserved safety keeps deletion, escaped violation invalidates it`
- `Shorten an existing skill` + `representative routes preserve behavior with fewer selected bytes`: `Deleted instructions are redundant with existing authority`; `Run the before/after route corpus with selected-byte measurement`; `same decisions with fewer bytes or a behavior mismatch`; `parity keeps shortening, mismatch restores the necessary instruction`
- `Delegate each issue to an isolated worktree owner` + `no parent edit and no cross-owner write`: `Every implementation path has exactly one owner`; `Compare the frozen issue path manifests before any branch setup`; `one owner per path or an overlap/unowned path`; `disjoint ownership opens setup, overlap redesigns the Wave`
- `Verify a package installation path` + `clean online install and offline cache reuse both succeed`: `The offline path uses only artifacts produced by the online install`; `Install online once, block network, and reinstall from the captured cache`; `offline success with identical identity or cache miss/identity drift`; `identical success keeps the plan, failure returns to package ownership`
- `Publish the implemented skill catalog` + `both READMEs list only installed skill frontmatter names and descriptions`: `Frontmatter alone supplies every catalog row`; `Enumerate installed skill frontmatter and compare it with both proposed README tables`; `exact row parity or missing/extra rows`; `parity keeps documentation update, drift blocks publication`
- `Preserve artifact trust while removing fixed SHA snapshots` + `tamper and rollback still fail using producer-provided metadata`: `Authentic producer metadata fully replaces fixed expected SHA values`; `Run valid, tampered, and rollback artifacts through the surviving verifier`; `valid passes and both attacks fail, or a trust boundary escapes`; `complete discrimination keeps pin removal, any escape invalidates it`

## Receipt

MUST return exactly these four newline-delimited fields:

```text
invalidating_assumption=<one simple declarative causal claim>
bounded_probe=<one imperative sentence for the smallest discriminating probe>
expected_observable=<one passing observation or one failing observation>
decision_effect=<passing observation keeps the plan, failure stops, narrows, or returns it>
```

MUST NOT add a status, verdict, approval, heading, checklist, or extra field.
MUST NOT fan out work, choose or change an owner, perform the probe, mutate
state, implement a repair, or issue a reviewer or completion judgment.

## Declines

When declining, MUST choose the first matching category in list order and return exactly its line:

- Routine direct fix: `Decline; this is a routine direct fix, not an important plan stress test.`
- Current-diff verdict: `Decline; selected review owns current-diff verdicts.`
- Completion proof: `Decline; proof-driven-completion owns completion evidence.`
- Broad concern list: `Decline; broad concern checklists are out of scope.`
- Multiple plans: `Decline; exactly one plan is required.`
- Owner or worktree choice: `Decline; orchestration owns routing and ownership.`
- Implementation request: `Decline; implementation and mutation are out of scope.`
- Approval request: `Decline; the skill returns an advisory receipt, not an approval verdict.`
- Missing acceptance criteria: `Decline; the required acceptance criteria are missing.`
- No explicit opt-in: `Decline; explicit user opt-in is required.`
<!-- dprint-ignore-end -->
