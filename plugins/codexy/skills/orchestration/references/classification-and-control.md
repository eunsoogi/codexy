# Classification And Control

## Parent And Child Boundary

- The plugin-invoking Codex thread is the orchestrator. It creates or confirms
  issues, assigns branches, delegates lanes, opens PRs when appropriate,
  performs parent verification, coordinates squash merge, and syncs `main`.
- A child Codex worktree thread owns implementation edits, local verification,
  and review-response fixes for its assigned issue or lane.
- `$planning` owns plan content, plan updates, and plan-file rules. Orchestration
  owns task classification, ownership, assignment, worktree, dispatch, execution
  coordination, and merge; a plan MUST NOT transfer those authorities.
- Durable delegation and multi-lane ownership preserve this ownership boundary
  but do not select a strict workflow profile by themselves; profile selection
  follows concrete risk, explicit audit, or materially shared integration risk.
- Independent requested outcomes MUST be decomposed into separate issue-sized
  atomic child lanes before child thread, worktree, branch, or PR creation.
- The root orchestrator MUST create, fork, or assign the owning child thread
  before implementation patches begin for any lane that needs a branch,
  worktree, PR, durable child context, or review-response ownership.
- The orchestrator MUST NOT directly fix child-owned review feedback unless a
  maintainer explicitly reassigns the lane to the orchestrator or the feedback
  belongs to the orchestrator's own scoped lane.
- If a child lane is bundled after dispatch or edits begin, MUST stop that lane,
  MUST preserve draft state, report the overlap, and MUST split independent
  outcomes into atomic issues, threads, worktrees, branches, and PRs before
  resuming.

## Compaction And Continuation

MUST treat loss of the active `@Codexy` or Codexy plugin workflow contract after
context compaction, goal continuation, or resume as a dogfooding defect.

Before editing after compaction or continuation, re-check the selected external
surface state when the task has an issue or PR. For a repository-owned lane,
also capture a fresh git preflight with:

```sh
pwd
git status --short --branch
git rev-parse HEAD
git rev-parse origin/main
git log --graph --oneline --decorate --all -n 12
```

If a summary omits duplicate/no-active-work issue state, PR state, parent/child
ownership, or authoritative stop condition, rebuild the evidence before editing.

## Child Execution Discipline

Child implementation threads assigned a non-trivial lane MUST run their own
execution loop instead of treating the parent handoff as permission for
unassigned or out-of-scope edits.

- MUST use the native goal tools for lane state: `create_goal`, `get_goal`, and
  `update_goal`. A native goal object and actual state/transition readback are
  mandatory for normal execution; prose-only `Goal:` text MUST NOT substitute
  for the goal API. If the native goal surface is unavailable, MUST report the
  exact limitation and required action, and MUST NOT start or claim execution
  that depends on it.
- MUST keep real todo/plan state current with `update_plan` or the active todo
  surface when available, updating statuses from discovery through handoff.
  Prose-only `Todo:` text is not proof of todo/plan tooling. Using only goal or
  only todo/plan is insufficient for non-trivial child lanes unless the missing
  tool is unavailable and reported with its fallback.
- MUST use multi-agent execution when the lane has independent research
  questions, disjoint implementation slices, parallel QA or verification, review
  gates, review-feedback validation, or separable non-trivial subtasks.
- A child implementation thread MAY spawn bounded first-level specialist helpers
  or Sentinel reviewers, but every helper or Sentinel MUST NOT spawn, delegate
  to, or create any additional agent, helper, reviewer, task, or thread.
- When a packaged Codexy specialist role is available and the task clearly falls
  within that specialist's stated scope, the child MUST use the matching
  specialist or record a concrete skip rationale tied to scope, atomicity,
  unavailable tooling, or lack of a matching task. It MUST NOT replace a
  required Codex child thread/worktree owner with a subagent helper.
- Specialist routing MUST include `codexy-cartographer` for repository, file,
  dependency, or ownership mapping; `codexy-architect` for boundary, schema,
  MCP, LSP, plugin architecture, or long-lived extension-point changes;
  `codexy-warden` for workflows, shell commands, credentials, remote MCP
  endpoints, untrusted input, repository permissions, install scripts, local
  state mutation, or generated evidence with security implications;
  `codexy-auditor` after implementation for acceptance-criteria, readiness, and
  observable verification passes across repository, CLI, config, GitHub,
  browser/desktop, documents/artifacts, spreadsheets/data, research/wiki,
  read-only/local, plugin, documentation, or workflow surfaces; a separately
  installed integration specialist for reconciling parallel lanes, conflict
  checks, main updates, or merge sequencing; `codexy-shipwright` for release,
  packaging, version, marketplace, manifest, tag, or rollback work; the optional
  `codexy-github` plugin's `codexy-weaver` for GitHub integration when
  installed; and the reviewer selected only by `review-profiles.md` for the
  final reviewer gate. `$planning` owns plan creation, updates, and plan-file
  rules; orchestration owns classification, ownership, assignment, worktree,
  dispatch, execution coordination, and merge. Generic owning children use the
  engineering workflow for technical design, diagnosis, TDD, QA, refactoring,
  implementation, and verification of one atomic issue, and directly own scoped
  documentation and handoff. They MUST NOT recreate removed specialists as
  aliases.
- If multi-agent tooling is available, "not useful" is acceptable only with a
  concrete rationale tied to atomicity, tiny scope, or the absence of separable
  work.
- If a required execution tool is unavailable, MUST report the exact limitation
  and required action. MUST NOT use a different tool or prose substitute to
  claim the same contract, and MUST NOT silently skip the discipline.
- Before handoff, PR readiness, completion, or parent acceptance, the child MUST
  follow the proportionate current-head contract in `review-profiles.md`:
  `light` has no LLM reviewer, `standard` uses
  `plugins/codexy/agents/codexy-inspector.toml` when an independent reviewer is
  required, and `strict` uses `plugins/codexy/agents/codexy-sentinel.toml`. The
  reviewer reads the current diff, exact head, lane scope, and relevant
  verification; missing historical or optional evidence is not a default gate.
- A selected reviewer MUST return `PASS`, `BLOCK`, or `UNOBSERVABLE` when it
  reaches a terminal result. A bounded wait with no result is `PENDING`, and an
  independently observed live reviewer is `RUNNING`; neither is a verdict. The
  owning lane MUST retain the same reviewer while it is pending and MUST NOT
  interrupt, replace, duplicate, or turn it into an automatic review stack.
- The compact current-head control state MUST bind the selected profile,
  reviewer when applicable, exact `reviewed_head`, actual result or status, and
  unresolved findings. It MUST NOT require review-count fields, ordered history,
  transcript reconstruction, quota bookkeeping, or a disposition ledger. A
  `PASS` with no findings supports the review gate; a stale head, failed
  relevant check, `BLOCK`, `UNOBSERVABLE`, or actual finding remains blocking.
- A second reviewer, broad recheck, semantic evaluator, connector review, or
  evidence artifact MUST be tied to an explicit requirement or concrete risk.

## Completion-Handoff Validation

Opening a PR is not completion when the requested outcome includes completion,
merge, default Codexy merge flow, or no explicit stop/wait/draft-only/
leave-open instruction. Validate completion claims that could otherwise stop at
an open PR with the active project's completion-handoff contract and current PR
state. This check is applicable only when the GitHub surface is selected.

If the handoff discusses addressed review feedback, MUST include GraphQL
`reviewThreads.nodes` in the PR state evidence. Addressed unresolved threads,
including outdated-but-fixed threads, remain invalid unless the report documents
an accepted no-change rationale.

A checked contract is the sole merge authorization; generic finish, completion,
silence, clean gates, and a ready PR are non-authoritative signals.

## Direct Review-State Handoff

The selected profile and reviewer remain the authority for review state. The
compact current-head control MUST carry the existing
`codexy.review-control-state.v1` schema, selected `profile`, the policy
`reviewer` when applicable, exact `reviewed_head`, one actual `terminal_result`
or non-terminal `status`, and `unresolved_findings`. A selected reviewer MUST
match the current PR head and profile policy. `PASS` with no unresolved
actionable findings is the only positive review result; `BLOCK`, `UNOBSERVABLE`,
`PENDING`, `RUNNING`, stale heads, failed relevant checks, and actual findings
MUST not be presented as readiness.

The compact path does not require `issue_number`, review counts, ordered
history, a prior control state, transcript import, invocation telemetry, or a
disposition object. The authenticated current PR snapshot remains authoritative
for repository, PR, base, and head identity. `previous_control_state` MUST
remain rejected when supplied to the producer; absence of a previous snapshot is
valid for the compact path.

A child-owned lane MUST send implementation or review-response fixes to its
owning child. The parent consumes the current result and retains merge or
publication authority. A pending reviewer stays with the same reviewer until the
real result arrives; it MUST NOT be interrupted, replaced, duplicated, or
converted into a new approval request.

Light retains its no-reviewer route. Headings, prose, and optional receipts MUST
NOT override direct current-head facts.
