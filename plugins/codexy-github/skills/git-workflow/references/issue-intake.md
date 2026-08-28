# Issue Intake

Before any Codexy-created issue mutation, the child MUST ask `$orchestration` to
apply its public **issue-intake receipt** contract and receive explicit parent
approval for the validated candidate. The child MUST send its parent one JSON
receipt with this exact shape:

```json
{
  "parent_approval": {
    "decision": "approved",
    "source_task_id": "REPLACE_WITH_ACTUAL_SOURCE_TASK_ID"
  },
  "classification": "issue_sized_defect",
  "reproduction": {
    "decision": "supported",
    "surface_kind": "real_producer",
    "surface": "REPLACE",
    "steps": ["REPLACE"],
    "observed": "REPLACE"
  },
  "ownership": {
    "decision": "cannot_own",
    "existing_owner": { "kind": "issue", "number": 195 },
    "rationale": "REPLACE"
  },
  "duplicate_search": {
    "states": ["open", "closed"],
    "search_terms": ["REPLACE"],
    "results": [{ "issue": 195, "state": "closed", "match_kind": "related" }],
    "conclusion": { "decision": "no_duplicate" }
  },
  "necessity": {
    "decision": "thin_harness_change_required",
    "rationale": "REPLACE"
  },
  "title": "Validated descriptive issue title",
  "body": "REPLACE",
  "labels": ["repository-label"],
  "repository_labels": ["repository-label"],
  "repository_milestones": ["repository-milestone"],
  "repository_assignees": ["repository-assignee"],
  "milestone": "repository-milestone",
  "assignee": "repository-assignee"
}
```

Every `REPLACE` value MUST contain actual source-task or evidence data; the
literal template is intentionally invalid. Before approval or mutation, the
canonical validator MUST pass:

```sh
scripts/validate-plugin-config.sh --check-issue-intake \
  --issue-intake-file <receipt.json>
```

The candidate evidence MUST come from a supported real producer or user-facing
surface and include:

- an all-state duplicate search and exact-versus-related classification;
- existing-owner exclusion and why a thin issue-sized change is necessary;
- a descriptive title and substantive problem, scope, acceptance, and
  verification content;
- the live repository label, milestone, and assignee taxonomies plus selected
  values; and
- the approving parent task identity and decision.

- The receipt body MUST contain substantive `## Problem`, `## Scope`,
  `## Acceptance Criteria`, and `## Verification` sections.
- `surface_kind` MUST be `real_producer` or `user_facing`.
- `existing_owner.kind` MUST be `issue` or `pull_request`.
- Duplicate search MUST cover `open` and `closed`. Each result uses
  `match_kind: exact` or `related`.
- An exact result MUST use
  `conclusion: {"decision":"duplicate", "canonical_issue":NUMBER}` and MUST be
  rejected before issue creation.
- `classification: unsupported_synthetic`,
  `classification: same_class_observation`, `ownership.decision: can_own`, and
  `necessity.decision: no_change` are handoff-only outcomes.
- Rationale wording MUST NOT override typed decisions. Reproduction, ownership,
  duplicate-search terms, and necessity evidence MUST be substantive.
- Repository label, milestone, and assignee taxonomies MUST be non-empty, and
  every selected value MUST belong to its taxonomy.

An exact duplicate, rejected or missing approval, unsupported synthetic or
same-class observation, ownable existing work, no-change outcome, invalid
metadata, or incomplete evidence MUST NOT create or mutate an issue. Preserve it
as a handoff-only result with the canonical existing issue when applicable.

Immediately before an approved mutation, MUST refresh duplicate and taxonomy
evidence. After mutation, MUST read back the issue number, URL, title, state,
labels, milestone, assignee, and body from GitHub. Connector or GitHub API
evidence is authoritative; a local receipt alone is not.
