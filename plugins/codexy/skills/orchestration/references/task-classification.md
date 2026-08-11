# Task Classification Within Orchestration

## Purpose

MUST classify first for any Codexy work to select a workflow profile
before acting. The versioned canonical contract
is [Workflow Profiles](workflow-profiles.json):
`light` is the default, `standard` scales proof for non-trivial single-owner
work, and `strict` applies to high-risk work. The contract's invariant floor
applies to every profile and profiles MUST NOT weaken merge gates.
Proof and Sentinel requirements scale with the profile, but every readiness
claim MUST remain bound to the current head.

Tiny or read-only work needs no visible receipt. Light and standard work MUST
remain proportionate: they MUST NOT require an exact eight-row table, goal/plan
receipts, multi-agent decisions, specialist skip rationales, or unavailable-tool
explanations merely because the profile was selected.

When profile metadata is recorded, MUST use one active-Markdown line of the form
`Workflow profile: light`, `Workflow profile: standard`, or `Workflow profile: strict`.
MUST use the same plain metadata form for `Durable delegation: yes`, `Multi-lane ownership: yes`,
or `Explicit audit evidence: requested`; an affirmative trigger makes the current lane strict.
MUST NOT reuse, duplicate, or place active workflow evidence inside fenced historical examples.

## Classification Workflow

1. Intake:
   - MUST read the latest user request, explicit issue or PR, repository
     instructions, and named skills.
   - MUST separate hard requirements, stop conditions, non-goals, and requested
     completion state.
   - MUST identify whether the request is already scoped to an issue, PR, branch,
     child thread, or worktree.
2. MUST classify TDD applicability before selecting an engineering workflow:
   - MUST use the machine-owned [TDD Classification Policy](tdd-classification-policy.json)
     to decide `engineering_tdd_required` from typed work boundaries.
   - Engineering boundaries require faithful RED/GREEN. These include production or
     runtime behavior, validators, parsers (including Markdown-backed parsers),
     hooks, CLIs, workflows, installers, package resolution, tools, executable
     contracts, defect repairs, and behavior-preserving refactors.
   - README, documentation, instruction-only skills, agent prompts, declarative
     metadata, issue/PR metadata, roadmap or release prose, inventories, diagrams,
     examples, and copy edits MUST NOT manufacture a RED. They require direct
     readback plus the applicable structural, link, render, frontmatter, package,
     or authentic-surface proof.
   - Mixed work MUST run RED/GREEN only for its engineering boundaries and MUST
     record proportional proof for its non-engineering boundaries.
   - The file format, strict profile, review gate, or presence of a Markdown file
     MUST NOT by itself require TDD.
3. MUST choose a profile:
   - MUST use `light` for read-only, documentation, tiny fixes, and ordinary
     single-owner mutations unless a strict trigger applies.
   - MUST use `standard` for non-trivial single-owner work needing added planning
     or proof.
   - MUST use `strict` for destructive, security, permission, secret, release,
     high-consequence external-state, high-risk guardrail, or merge-sensitive work.
   - Durable delegation, multi-lane ownership, and explicit audit evidence MUST
     escalate to strict even when light or standard was requested.
4. MUST classify strict work:
   - MUST pick one primary lane type from the taxonomy below.
   - MUST pick any secondary surface that affects verification, such as plugin
     packaging, GitHub state, docs, validators, MCP, LSP, release, or browser.
   - MUST decide owner as `parent-owned`, `child-owned`, `current-thread-owned`, or
     `external/human-owned`.
5. MUST route strict work:
   - MUST name the required Codexy skills and any explicit user-named skills.
   - MUST name required tool surfaces, including goal, plan/todo, codegraph, LSP,
     GitHub, validators, local tests, and packaged `codexy-sentinel`.
   - MUST decide whether multi-agent helper work or separate Codex
     thread/worktree ownership is required, not useful, or unavailable. MUST treat
     them as different surfaces: subagents may assist with bounded research,
     review, or worker tasks, but they are not child-owned Codex
     subthread/worktree owners for issue-sized lanes that need a branch,
     durable worktree, PR, or review-response ownership. A `tool_search` miss
     alone is not proof that Codex thread/worktree tooling is unavailable when
     another real surface has produced `thread/start` or `turn/start` events;
     MUST record that as a discovery/exposure mismatch and keep routing through the
     real thread surface.
   - When packaged Codexy specialist subagents are available, required
     tools/evidence MUST name the specialist roles whose stated scope clearly
     matches the task or the concrete rationale for skipping them. It MUST NOT
     treat specialist subagent use as the child thread/worktree owner for an
     issue-sized lane.
6. Gate:
   - State the first allowed action after classification.
   - If classification exposes missing scope, missing issue/PR identity,
     conflicting owner, bundled lanes, or unavailable required tools, MUST stop and
     MUST report the blocker before setup or edits.

## Lane Taxonomy

- `orchestration/lane setup`: issue sizing, owner routing, branch/worktree or
  child-thread setup, delegation packet creation, or parent coordination.
- `implementation`: code, skill, validator, documentation, configuration, or
  workflow changes owned by the current implementation lane.
- `review response`: responding to automated or human review, review
  threads, inline comments, or PR feedback on an existing branch.
- `GitHub/merge`: PR creation, PR update, review request, label changes,
  branch protection, merge gate inspection, squash merge, branch deletion, or
  post-merge main sync.
- `validation/QA`: local verification, proof bundle creation, acceptance
  checks, plugin validation, UI/manual QA, or current-head evidence audit.
- `documentation/skill authoring`: `AGENTS.md`, `README`, skill instruction,
  handoff, runbook, prompt, or policy authoring where the durable behavior is
  instructional.
- `plugin/release`: manifest, marketplace, install surface, MCP/LSP package,
  version sync, release notes, artifact, tag, publish, or rollback work.
- `investigation/debugging`: failure reproduction, root-cause analysis,
  regression triage, unexpected tool behavior, or flake diagnosis.
- `issue/intake only`: issue creation, scoping, labeling, acceptance criteria,
  or question-answering without implementation setup.
- `other`: only when none of the above apply; explain why and define the
  equivalent workflow gates before proceeding.

## Owner Decision Rules

- MUST choose `child-owned` when the request needs its own branch, worktree, PR,
  long-running implementation context, or review-response patches for a
  delegated lane.
- MUST choose `parent-owned` for orchestration, issue setup, delegation packets,
  PR/review/merge coordination, or parent verification of child evidence.
- MUST choose `current-thread-owned` only when the active thread is explicitly the
  owning implementation lane for the issue-sized work.
- MUST choose `external/human-owned` when the next action depends on a maintainer,
  GitHub permission, external service, secret, or manual decision.
- If owner choice is ambiguous, MUST ask or stop with a classification blocker
  before branch/worktree setup or edits.
- Subagents are not child-owned implementation owners. They can assist bounded
  research, review, or QA, but they MUST NOT satisfy a required Codex
  thread/worktree owner for an issue-sized implementation lane.
- MUST NOT classify `spawn_agent`, `multi_agent`, specialist, reviewer, or
  worker delegation as a Codex subthread/worktree owner. If true
  thread/worktree tooling is required but unavailable, record the exposure
  blocker instead of satisfying ownership with a subagent.

## Authority Boundary

Strict work, durable delegation, multi-lane ownership, and audit evidence use
the formal contract. The GFM table is presentation/audit evidence only. The
authoritative ownership record MUST be the two non-Markdown lines immediately
before `Task classification:`:

```text
Ownership metadata source: parent-supplied
Lane ownership: child-owned
Task classification:
```

For an undelegated lane, agents MUST replace `parent-supplied` with
`current-thread-classified` and MUST use exactly one owner value:
`parent-owned`, `child-owned`, `current-thread-owned`, or
`external/human-owned`. A delegated child lane MUST use the parent-supplied
form. The GFM `Owner decision` cell is display/audit evidence and MUST NOT be
used as control authority. Its value MUST use the typed grammar
`affirmative <owner>` or `denied <owner>`, optionally followed by
`because <opaque rationale>`; `<owner>` MUST be exactly one of the owner
values above. An affirmative display owner MUST equal the authoritative
metadata owner before it can support a classification. Rationale text is not
authority and MUST NOT determine affirmation or selection.

## Required Output

For light and standard, select the profile internally and state only the outcome
needed to coordinate the work. Tiny/read-only work MUST NOT emit a receipt.

For strict work or a formal-evidence trigger, MUST emit the authoritative
ownership record, then render exactly one ordered two-column GFM table before
the first workflow action:

| Field | Value |
| --- | --- |
| Lane type | One primary taxonomy entry. |
| Secondary surfaces | Relevant secondary surfaces or `None`. |
| Owner decision | `affirmative <owner>` with an optional `because <rationale>`, or `denied <owner>`. |
| Atomic scope | Whether the request is issue-sized, bundled, or needs splitting before setup. |
| Required skills | The Codexy skills to read before acting. |
| Required tools/evidence | Lane-relevant required evidence and unavailable-tool fallbacks. |
| First allowed action | The next concrete action allowed only after classification. |
| Stop/blocker | `None` or the exact blocker that prevents proceeding. |

## Formal Classification Output

- The `Lane type` row names one primary taxonomy entry.
- The `Secondary surfaces` row names relevant secondary surfaces or `None`.
- The `Owner decision` row MUST use the typed affirmation and exact owner grammar;
  its optional rationale explains the decision but does not create authority.
- The `Atomic scope` row states whether the request is issue-sized, bundled, or needs
  splitting before setup.
- The `Required skills` row lists the Codexy skills to read before acting.
- The `Required tools/evidence` row lists lane-relevant required evidence, including
  unavailable-tool fallbacks where a relevant Codexy tool, GitHub surface,
  validator, test, LSP, codegraph, goal/plan, or reviewer gate is unavailable.
- The `First allowed action` row states the next concrete action that may happen only
  after this classification.
- The `Stop/blocker` row states `None` or the exact blocker that prevents proceeding.

## Gates

- Missing formal classification evidence blocks strict setup, delegation,
  multi-lane work, audit evidence, release work, and PR-readiness claims.
- Child lanes MUST emit the complete orchestration table before
  creating or switching to an implementation branch or worktree. Handoff
  evidence MUST NOT report child-created branch/worktree setup before that
  table; `scripts/validate-plugin-config --check-child-lane-ownership
  --evidence-file <path>` catches this workflow defect. Issue #231 tracks the
  exact dogfood evidence from issue #228: child branch
  `codexy/228-reject-generic-reviewer-gate-sentinel-proof` was created
  immediately after thread rename and before formal `$orchestration`
  evidence.
- Formal classification MUST happen before acting on or using the owner decision to
  edit files, set up branches or worktrees, delegate lanes, or route review
  feedback.
- Classification MUST NOT waive root `AGENTS.md`, user stop conditions,
  selected skills, unresolved review-thread gates, or packaged `codexy-sentinel`.
- A broad or bundled request MUST be split into atomic lanes before any
  implementation lane begins unless a maintainer explicitly scopes it as one
  atomic lane before edits.
- If the classified lane touches plugin packaging, skills, validators, MCP,
  LSP, release, or GitHub surfaces, include the matching validation or external
  observation in required evidence.
- If thread/worktree tool discovery is part of the classification, required
  evidence MUST distinguish true Codex thread evidence such as
  `thread/start`/`turn/start` or `codex_app` thread tools from subagents,
  GitHub review-thread tools, and `codex` CLI commands. `codex exec`,
  `codex fork`, and generic `codex app-server` commands are not fallback
  substitutes for a required thread owner.

## Failure Modes

- Selecting light or standard to bypass a formal-evidence trigger.
- Creating a branch or worktree before deciding whether the lane is
  parent-owned or child-owned.
- Creating or switching to a child implementation branch or worktree after a
  thread rename but before the complete orchestration table.
- Treating review response, merge, validation, release, and implementation as
  one generic task.
- Letting a parent patch a child-owned implementation or review-response lane.
- Reporting a PR-ready handoff without classification evidence in the proof
  bundle.
