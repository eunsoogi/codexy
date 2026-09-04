# Classification And Control

## Parent And Child Boundary

- The plugin-invoking Codex thread is the orchestrator. It creates or confirms
  issues, assigns branches, delegates lanes, opens PRs when appropriate,
  performs parent verification, coordinates squash merge, and syncs `main`.
- A child Codex worktree thread owns implementation edits, local verification,
  and review-response fixes for its assigned issue or lane.
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
execution loop instead of treating the parent handoff as permission for ad hoc
edits.

- MUST use real goal tools when available. MUST use `create_goal`, `get_goal`,
  and `update_goal` for lane state; prose-only `Goal:` text is fallback
  documentation, not proof of goal-tool use. If goal tooling is unavailable,
  MUST keep a visible textual goal with success criteria, update it as evidence
  changes, and report the unavailable-tool fallback in handoff evidence.
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
  final reviewer gate. Orchestration owns planning; generic owning children use
  the engineering workflow for diagnosis, TDD, QA, and refactoring and directly
  own scoped implementation, documentation, and handoff. They MUST NOT recreate
  removed specialists as aliases.
- If multi-agent tooling is available, "not useful" is acceptable only with a
  concrete rationale tied to atomicity, tiny scope, or the absence of separable
  work.
- If a required execution tool is unavailable, say so in the thread and use the
  closest available fallback. MUST NOT silently skip the discipline.
- Before handoff, PR readiness, completion, or parent acceptance, the child MUST
  follow the one-reviewer contract in `review-profiles.md`: light has no LLM
  reviewer, standard runs `plugins/codexy/agents/codexy-inspector.toml`, and
  strict runs `plugins/codexy/agents/codexy-sentinel.toml` against the current
  diff, exact head or file state, lane scope, touched implementation-file LOC
  evidence, verification outputs, and available evidence.
- Packaged Sentinel terminal results MUST be `PASS`, `BLOCK`, or `UNOBSERVABLE`.
  A bounded wait with no event is a non-terminal `PENDING` observation, and an
  independently observed live reviewer is `RUNNING`; neither observation is a
  reviewer verdict or fallback-eligible. The owning lane MUST retain the same
  reviewer and wait for its natural terminal result. The child MUST NOT
  interrupt, replace, or duplicate it and MUST keep push/readiness blocked until
  that result arrives.
- The owner MUST carry one issue-wide maximum of three terminal profile-selected
  verdicts across fresh goals, repair stages, compaction, parent
  reauthorization, and reviewer-route resets. A terminal `PASS`, `BLOCK`, or
  `UNOBSERVABLE` counts; `PENDING` and `RUNNING` do not. The direct control
  state MUST carry `issue_number`, `terminal_review_count`, the policy's
  `terminal_review_limit`, and an ordered `terminal_review_history`; the history
  MUST be carried forward rather than reset or shortened at a lane boundary.
  After the third verdict, the owner MUST NOT invoke another profile-selected
  reviewer. A third `BLOCK` permits one bounded repair of its
  issue-contract/root findings and exact-head proof before handoff. A third
  `UNOBSERVABLE` requires a maintainer-owned final disposition with current
  proof. Neither path waives tests, validators, CI, review-thread, ownership,
  safety, LOC, or merge gates, and neither may set a goal to `blocked` because
  of quota exhaustion or a wait.

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

The selected profile and its reviewer remain the authority for review state. The
current-head control state MUST preserve the existing
`codexy.review-control-state.v1` schema and carry `profile`, `reviewer`,
`reviewed_head`, `terminal_result`, `unresolved_findings`, `full_review_count`,
`delta_review_count`, `issue_number`, `terminal_review_count`,
`terminal_review_limit`, and `terminal_review_history` directly.

For standard and strict profiles, the reviewer and `reviewed_head` MUST match
the current PR state, `terminal_result` MUST be exactly `PASS`, `BLOCK`, or
`UNOBSERVABLE`, and a readiness handoff MUST have `PASS`, no unresolved
findings, one full review, and at most one delta review. The history MUST
contain that one `full` event, optionally followed by one `delta` event, with
unique review IDs, the selected reviewer on every event, and a different
reviewed head for each event. Its length MUST equal `terminal_review_count`, and
the full and delta counters MUST equal the corresponding event kinds.

The one bounded post-cap path is a third `required_current_head` event after the
full and delta events. It MUST keep the same selected reviewer, bind the current
head, set `terminal_review_count` to three, and carry exactly one
`post_cap_re_review` object with `reason` set to either
`mandatory_base_integration` or `in_scope_contract_root_repair`, plus
`prior_reviewed_head` equal to the delta head. It MUST also carry
`qualifying_change.from_head`, `qualifying_change.to_head`, and
`qualifying_change.evidence_commit`; those values MUST bind the delta head and
current head, and the evidence commit MUST be in their Git ancestry. The current
head MUST differ from that prior head. Optional churn, a fourth event, a
duplicate head or ID, a truncated/reordered history, and a marker on a non-third
event MUST be rejected.

Every reviewer-backed transition MUST use authenticated current and previous PR
snapshots from the canonical GitHub readback producer. Each snapshot MUST bind
the same PR repository, number, URL, base branch, and authenticated capture
provenance, and MUST carry `baseRefOid` and `headRefOid`. The previous
snapshot's direct `reviewControl` is the only predecessor authority; a
separately supplied `previous_control_state` MUST be rejected. The first full
review appends to a clean genesis with zero terminal reviews, and later states
MUST preserve the exact prior history prefix and increment the terminal count by
one. The current snapshot supplies the current head and base identity; the
validator MUST NOT rewrite either from caller-supplied review control.

For `mandatory_base_integration`, the previous and current `baseRefOid` values
MUST differ, the current base MUST descend from the previous base, and the
integration evidence MUST descend from the current base. For
`in_scope_contract_root_repair`, the base OID MUST remain unchanged, the prior
delta MUST be `BLOCK` with non-empty findings, and
`qualifying_change.finding_ids` MUST exactly identify those findings. In both
cases the evidence commit MUST descend from the prior delta and precede the
current head; a root-repair evidence commit MUST change the reviewed tree.
Arbitrary JSON agreement is not authenticated readback authority.

Light retains its existing no-reviewer route and MUST NOT carry terminal review
history or post-cap fields. A third `BLOCK` or `UNOBSERVABLE` remains a terminal
non-PASS disposition; the post-cap path never turns it into readiness.

Headings, field order, explanatory prose, and omitted legacy ceremony fields
MUST NOT override those direct facts. The ordered history and qualifying-change
evidence are part of the direct control state; no auxiliary review ledger or
replacement schema is needed. The selected reviewer MUST remain active and
unchanged: the owner MUST NOT duplicate, poll, interrupt, or replace that
reviewer while waiting for a terminal result.
