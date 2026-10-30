# Classification And Control

## Task Classification Decision

MUST use
[task classification](task-classification.md#orchestration-request-ownership) as
the single source for task kind, scoped authority, implementation owner, and
dependency/resource/integration decisions. This reference MUST consume that
decision and MUST NOT restate or override it. The remaining guidance applies
after the owner and route are established.

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
- Before handoff, PR readiness, completion, or parent acceptance, MUST follow
  the selected [review profile](review-profiles.md) and
  [review lifecycle](review-lifecycle.md). Check selection, verification depth,
  and test-oracle preservation MUST follow
  [quality assurance](../../engineering/references/quality-assurance.md).
  Execution-evidence validity, reuse, and outcome judgment MUST follow
  [proof-driven completion](../../proof-driven-completion/SKILL.md). These
  consumers MUST NOT redefine the canonical owners' policies.

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

Direct review-state handoffs MUST use the existing runtime-validated
`codexy.review-control-state.v1` contract and follow
[review profiles](review-profiles.md) and
[review lifecycle](review-lifecycle.md). Outcome claims MUST also follow
[proof-driven completion](../../proof-driven-completion/SKILL.md). The compact
consumer instructions MUST NOT redefine schema fields, reviewer status meanings,
or current-head binding.
