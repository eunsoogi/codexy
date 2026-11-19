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

Before editing after compaction or continuation, follow
[Dreaming's current-state recovery](../../self-improvement/references/recovery.md#refresh-current-state)
to decide whether a live external surface needs refresh and which one. This
section MUST NOT add a second repository, PR, review, or release inventory. The
canonical [goal-transition handoff contract](goal-transition-reporting.md) owns
terminal handoff fields and delivery.

## Child Execution Discipline

Child implementation threads assigned a non-trivial lane MUST run their own
execution loop instead of treating the parent handoff as permission for
unassigned or out-of-scope edits.

- Assignment objective, scope, evidence, stop condition, and parent are lane
  data. A child MUST NOT be required to use native goal tools. Explicitly
  managed goals retain their host-controlled state and transition rules.
- MUST keep real todo/plan state current with `update_plan` or the active todo
  surface when available, updating statuses from discovery through handoff.
  Prose-only `Todo:` text is not proof of todo/plan tooling. Report an
  unavailable todo surface and its fallback.
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
  installed; and optional final Sentinel review only under `review-profiles.md`.
  Sentinel MUST NOT be used for intermediate review. `$planning` owns plan
  creation, updates, and plan-file rules; orchestration owns classification,
  ownership, assignment, worktree, dispatch, execution coordination, and merge.
  Generic owning children use the engineering workflow for technical design,
  diagnosis, TDD, QA, refactoring, implementation, and verification of one
  atomic issue, and directly own scoped documentation and handoff. They MUST NOT
  recreate removed specialists as aliases.
- If multi-agent tooling is available, "not useful" is acceptable only with a
  concrete rationale tied to atomicity, tiny scope, or the absence of separable
  work.
- If a required execution tool is unavailable, MUST report the exact limitation
  and required action. MUST NOT use a different tool or prose substitute to
  claim the same contract, and MUST NOT silently skip the discipline.
- Before handoff, PR readiness, completion, or parent acceptance, MUST follow
  the [optional final review](review-profiles.md) decision and
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

Merge authority MUST follow the Git Workflow
[merge boundary](../../../../codexy-github/skills/git-workflow/SKILL.md#merge-boundary).
A selected contract may add prerequisites but does not replace host/connector
authorization or GitHub permissions. Generic finish, completion, silence, clean
gates, and a ready PR are not authorization.

## Direct Review-State Handoff

Direct review-state handoffs MUST use the existing runtime-validated current
review-control contract and follow [optional final review](review-profiles.md)
and [review lifecycle](review-lifecycle.md). Outcome claims MUST also follow
[proof-driven completion](../../proof-driven-completion/SKILL.md). The compact
consumer instructions MUST NOT redefine schema fields, reviewer status meanings,
or current-head binding.
