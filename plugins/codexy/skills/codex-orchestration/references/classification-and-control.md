# Classification And Control

## Parent And Child Boundary

- The plugin-invoking Codex thread is the orchestrator. It creates or confirms
  issues, assigns branches, delegates lanes, opens PRs when appropriate,
  requests Codex review, performs parent verification, coordinates squash
  merge, and syncs `main`.
- A child Codex worktree thread owns implementation edits, local verification,
  and review-response fixes for its assigned issue or lane.
- Independent requested outcomes MUST be decomposed into separate issue-sized
  atomic child lanes before child thread, worktree, branch, or PR creation.
- The orchestrator MUST create, fork, or assign the owning child thread before
  implementation patches begin for any lane that needs a branch, worktree, PR,
  durable child context, or review-response ownership.
- The orchestrator MUST NOT directly fix child-owned review feedback unless a
  maintainer explicitly reassigns the lane to the orchestrator or the feedback
  belongs to the orchestrator's own scoped lane.
- If a child lane is bundled after dispatch or edits begin, MUST stop that lane,
  MUST preserve draft state, report the overlap, and MUST split independent outcomes
  into atomic issues, threads, worktrees, branches, and PRs before resuming.

## Compaction And Continuation

MUST treat loss of the active `@Codexy` or Codexy plugin workflow contract after
context compaction, goal continuation, or resume as a dogfooding defect.

Before editing after compaction or continuation, re-check GitHub state for the
issue and PR. Also capture a fresh git preflight with:

```sh
pwd
git status --short --branch
git rev-parse HEAD
git rev-parse origin/main
git log --graph --oneline --decorate --all -n 12
```

If a summary omits duplicate/no-active-work issue state, PR state,
parent/child ownership, or authoritative stop condition, rebuild the evidence
before editing.

## Child Execution Discipline

Child implementation threads assigned a non-trivial lane MUST run their own
execution loop instead of treating the parent handoff as permission for ad hoc
edits.

- MUST use real goal tools when available. MUST use `create_goal`, `get_goal`, and
  `update_goal` for lane state; prose-only `Goal:` text is fallback
  documentation, not proof of goal-tool use. If goal tooling is unavailable,
  MUST keep a visible textual goal with success criteria, update it as evidence
  changes, and report the unavailable-tool fallback in handoff evidence.
- MUST keep real todo/plan state current with `update_plan` or the active todo
  surface when available, updating statuses from discovery through handoff.
  Prose-only `Todo:` text is not proof of todo/plan tooling. Using only goal
  or only todo/plan is insufficient for non-trivial child lanes unless the
  missing tool is unavailable and reported with its fallback.
- MUST use multi-agent execution when the lane has independent research questions,
  disjoint implementation slices, parallel QA or verification, review gates,
  review-feedback validation, or separable non-trivial subtasks.
- A child implementation thread MAY spawn bounded first-level specialist helpers or
  Sentinel reviewers, but every helper or Sentinel MUST NOT spawn, delegate to, or create any additional agent, helper, reviewer, task, or thread.
- When a packaged Codexy specialist role is available and the task clearly
  falls within that specialist's stated scope, the child MUST use the matching
  specialist or record a concrete skip rationale tied to scope, atomicity,
  unavailable tooling, or lack of a matching task. It MUST NOT replace a
  required Codex child thread/worktree owner with a subagent helper.
- Specialist routing MUST include `codexy-cartographer` for repository, file,
  dependency, or ownership mapping; `codexy-pathfinder` for planning or
  approach selection; `codexy-architect` for boundary, schema, MCP, LSP,
  plugin architecture, or long-lived extension-point changes; `codexy-tracer`
  for root-cause or failing behavior; `codexy-warden` for workflows, shell
  commands, credentials, remote MCP endpoints, untrusted input, repository
  permissions, install scripts, local state mutation, or generated evidence
  with security implications; `codexy-auditor` after implementation for
  acceptance-criteria, readiness, and observable verification passes across
  CLI, config, GitHub, browser, app, plugin, documentation, or workflow
  surfaces;
  `codexy-scribe` for docs, handoff, PR, release note, or workflow drafting;
  `codexy-forge` for scoped implementation edits after issue, branch, worktree,
  plan, and acceptance criteria are clear; `codexy-weaver` for reconciling
  parallel lanes, conflict checks, main updates, or merge sequencing;
  `codexy-sculptor` for refactor-heavy or LOC-boundary work;
  `codexy-shipwright` for release, packaging, version, marketplace, manifest,
  tag, or rollback work; and `codexy-sentinel` for the final reviewer gate.
- If multi-agent tooling is available, "not useful" is acceptable only with a
  concrete rationale tied to atomicity, tiny scope, or the absence of separable
  work.
- If a required execution tool is unavailable, say so in the thread and use the
  closest available fallback. MUST NOT silently skip the discipline.
- Before handoff, PR readiness, completion, or parent acceptance, the child
  MUST run `plugins/codexy/agents/codexy-sentinel.toml` against the current
  diff, exact head or file state, lane scope, touched implementation-file LOC
  evidence, verification outputs, and available evidence.
- Packaged Sentinel waits MUST end in `PASS`, `BLOCK`, or `UNOBSERVABLE`
  status. The child MUST keep push/readiness blocked for `BLOCK` or
  `UNOBSERVABLE`, and MUST NOT treat delayed, pending, stuck, or unobservable
  Sentinel output as approval without explicit maintainer fallback approval.

## Completion-Handoff Validation

Opening a PR is not completion when the requested outcome includes completion,
merge, default Codexy merge flow, or no explicit stop/wait/draft-only/
leave-open instruction. Validate completion claims that could otherwise stop at
an open PR:

```sh
scripts/validate-plugin-config --check-completion-handoff \
  --handoff-file <report> \
  --pr-state-file <gh-pr-view-json>
```

If the handoff discusses addressed review feedback, MUST include GraphQL
`reviewThreads.nodes` in the PR state evidence. Addressed unresolved threads, including
outdated-but-fixed threads, remain invalid unless the report documents an
accepted no-change rationale.
