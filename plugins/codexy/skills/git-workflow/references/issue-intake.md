# Issue Intake

Before a child creates a GitHub issue, it MUST send its parent one JSON receipt
with this exact shape, receive approval, and pass `--check-issue-intake`:

```json
{
  "parent_approval": {
    "decision": "approved",
    "source_task_id": "00000000-0000-0000-0000-000000000000"
  },
  "classification": "issue_sized_defect",
  "reproduction": {
    "decision": "supported",
    "surface_kind": "real_producer",
    "surface": "Codex app delegated child task",
    "steps": ["Reproduce the behavior on the supported surface"],
    "observed": "Record the behavior observed on that supported surface"
  },
  "ownership": {
    "decision": "cannot_own",
    "existing_owner": {"kind": "issue", "number": 195},
    "rationale": "Explain why the existing issue does not own the correction"
  },
  "duplicate_search": {
    "states": ["open", "closed"],
    "search_terms": ["recorded search terms"],
    "results": [
      {"issue": 195, "state": "closed", "match_kind": "related"}
    ],
    "conclusion": {"decision": "no_duplicate"}
  },
  "necessity": {
    "decision": "thin_harness_change_required",
    "rationale": "Explain why a narrow change is preferable to no change"
  },
  "title": "Validated descriptive issue title",
  "body": "## Problem\n...\n## Scope\n...\n## Acceptance Criteria\n...\n## Verification\n...",
  "labels": ["repository-label"],
  "repository_labels": ["repository-label"],
  "repository_milestones": ["repository-milestone"],
  "repository_assignees": ["repository-assignee"],
  "milestone": "repository-milestone",
  "assignee": "repository-assignee"
}
```

- `surface_kind` MUST be `real_producer` or `user_facing`.
- `existing_owner.kind` MUST be `issue` or `pull_request`.
- Duplicate search MUST cover `open` and `closed`. Each result uses
  `match_kind: exact` or `related`.
- An exact result MUST use `conclusion: {"decision":"duplicate",
  "canonical_issue":NUMBER}` and MUST be rejected before issue creation.
- `classification: unsupported_synthetic`, `classification:
  same_class_observation`, `ownership.decision: can_own`, and
  `necessity.decision: no_change` are handoff-only outcomes.
- Rationale wording MUST NOT override typed decisions. Reproduction, ownership,
  duplicate-search terms, and necessity evidence MUST be substantive.
- Repository label, milestone, and assignee taxonomies MUST be non-empty, and
  every selected value MUST belong to its taxonomy.
