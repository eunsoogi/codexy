# Context retention and safety

The runtime retains safety state on every handoff: issue and PR identity, owner
and worktree, base and head, dirty-index state, checks, unresolved review
threads, selected reviewer state, verification, external gates, and the next
action. Omitted safety state is typed and never treated as proof of absence.

Task references are selected from the closed task, surface, and risk routes in
the runtime handoff contract. Unknown, ambiguous, high-risk, security,
permission, and release classifications fail closed through `child_routing`.

## Ordered reference-route contract

When producing a `StableHandoff`, set `selected_references` to the exact ordered
list produced by this contract. These are route identifiers, not file paths, and
the validator compares the complete list including its order.

For a valid structured classification, use this procedure:

1. If the workflow is fail-closed, a surface list is empty, a workflow or
   surface or risk is unknown, or any risk is present, start with the fallback
   route. Otherwise, start with the workflow route and append each surface route
   in the input order.
2. Append each known risk route in the input order. An unknown risk contributes
   no route after causing fail-closed handling.
3. Keep the first occurrence of each identifier and discard later duplicates.

A legacy classification uses only its workflow route. A legacy fail-closed
classification uses the fallback route. An unknown legacy workflow is invalid.

The routes are:

### Fallback route

`workflow_profiles` → `task_classification` → `child_routing`

### Workflow routes

- `orchestration/lane setup`: `workflow_profiles` → `task_classification` →
  `tdd_classification_policy` → `child_routing` → `execution_budget` →
  `public_extension_contracts`
- `implementation`: `workflow_profiles` → `task_classification` →
  `tdd_classification_policy` → `execution_budget` → `proof_completion`
- `review response`: `workflow_profiles` → `task_classification` →
  `tdd_classification_policy` → `review_profiles` → `review_lifecycle` →
  `proof_completion` → `public_extension_contracts`
- `GitHub/merge`: `workflow_profiles` → `task_classification` →
  `tdd_classification_policy` → `review_profiles` → `review_lifecycle` →
  `proof_completion` → `public_extension_contracts`
- `validation/QA`: `workflow_profiles` → `task_classification` →
  `tdd_classification_policy` → `execution_budget` → `review_profiles` →
  `proof_completion`
- `documentation/skill authoring`: `workflow_profiles` → `task_classification` →
  `tdd_classification_policy` → `proof_completion`
- `plugin/release`: `workflow_profiles` → `task_classification` →
  `tdd_classification_policy` → `execution_budget` → `proof_completion` →
  `public_extension_contracts`
- `investigation/debugging`: `workflow_profiles` → `task_classification` →
  `tdd_classification_policy` → `execution_budget` → `proof_completion`
- `issue/intake only`: `workflow_profiles` → `task_classification` →
  `tdd_classification_policy` → `child_routing` → `public_extension_contracts`
- `other`: `workflow_profiles` → `tdd_classification_policy` → `child_routing`

### Surface routes

- `repository engineering`: `proof_completion`
- `GitHub`: `review_profiles` → `review_lifecycle` → `proof_completion` →
  `public_extension_contracts`
- `browser/desktop`: `workflow_profiles` → `task_classification` →
  `proof_completion`
- `documents/artifacts`: `workflow_profiles` → `task_classification` →
  `proof_completion`
- `spreadsheets/data`: `workflow_profiles` → `task_classification` →
  `proof_completion`
- `research/wiki`: `dreaming`
- `read-only/local`: `task_classification`

### Risk routes

- `mixed`: `workflow_profiles` → `task_classification` → `child_routing`
- `security`: `workflow_profiles` → `task_classification` → `child_routing` →
  `proof_completion`
- `permission`: `workflow_profiles` → `task_classification` → `child_routing` →
  `proof_completion`
- `destructive`: `workflow_profiles` → `task_classification` → `child_routing` →
  `proof_completion`
- `external_mutation`: `child_routing` → `proof_completion` →
  `public_extension_contracts`

Stable handoff identity covers the workflow classification and selected
references. Volatile identity covers the current safety and verification state.
Full conversation, full tool bodies, and full agent trees are never forwarded.

The executable route and retention contract is maintained by the packaged
runtime validator.
