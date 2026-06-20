---
name: task-classification
description: Use first for incoming Codexy work to classify the lane type, owner, required skills, evidence, and first allowed action before setup, delegation, implementation, PR handling, review response, or merge work begins.
---

# Task Classification

## Purpose

Run this skill first for any Codexy work. Classification is the gate that
decides which workflow may start, who owns the lane, which skills and tools are
required, what evidence will prove readiness, and the first allowed action.

Do not start branch or worktree setup, implementation edits, delegation, PR
handling, merge work, review-response routing, validation, release work, or
plugin repair until classification evidence exists in the thread.

## Classification Workflow

1. Intake:
   - Read the latest user request, explicit issue or PR, repository
     instructions, and named skills.
   - Separate hard requirements, stop conditions, non-goals, and requested
     completion state.
   - Identify whether the request is already scoped to an issue, PR, branch,
     child thread, or worktree.
2. Classify:
   - Pick one primary lane type from the taxonomy below.
   - Pick any secondary surface that affects verification, such as plugin
     packaging, GitHub state, docs, validators, MCP, LSP, release, or browser.
   - Decide owner as `parent-owned`, `child-owned`, `current-thread-owned`, or
     `external/human-owned`.
3. Route:
   - Name the required Codexy skills and any explicit user-named skills.
   - Name required tool surfaces, including goal, plan/todo, codegraph, LSP,
     GitHub, validators, local tests, and packaged `codexy-sentinel`.
   - Decide whether multi-agent or separate Codex thread/worktree ownership is
     required, not useful, or unavailable.
4. Gate:
   - State the first allowed action after classification.
   - If classification exposes missing scope, missing issue/PR identity,
     conflicting owner, bundled lanes, or unavailable required tools, stop and
     report the blocker before setup or edits.

## Lane Taxonomy

- `orchestration/lane setup`: issue sizing, owner routing, branch/worktree or
  child-thread setup, delegation packet creation, or parent coordination.
- `implementation`: code, skill, validator, documentation, configuration, or
  workflow changes owned by the current implementation lane.
- `review response`: responding to Codex connector, human review, review
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

- Choose `child-owned` when the request needs its own branch, worktree, PR,
  long-running implementation context, or review-response patches for a
  delegated lane.
- Choose `parent-owned` for orchestration, issue setup, delegation packets,
  PR/review/merge coordination, or parent verification of child evidence.
- Choose `current-thread-owned` only when the active thread is explicitly the
  owning implementation lane for the issue-sized work.
- Choose `external/human-owned` when the next action depends on a maintainer,
  GitHub permission, external service, secret, or manual decision.
- If owner choice is ambiguous, ask or stop with a classification blocker
  before branch/worktree setup or edits.

## Required Output

Use this shape before taking the first workflow action:

```text
Task classification:
Lane type:
Secondary surfaces:
Owner decision:
Atomic scope:
Required skills:
Required tools/evidence:
First allowed action:
Stop/blocker:
```

## Classification Output

- `Lane type:` names one primary taxonomy entry.
- `Secondary surfaces:` names relevant secondary surfaces or `None`.
- `Owner decision:` names the owner and why that owner is allowed to act.
- `Atomic scope:` states whether the request is issue-sized, bundled, or needs
  splitting before setup.
- `Required skills:` lists the Codexy skills to read before acting.
- `Required tools/evidence:` lists lane-relevant required evidence, including
  unavailable-tool fallbacks where a relevant Codexy tool, GitHub surface,
  validator, test, LSP, codegraph, goal/plan, or reviewer gate is unavailable.
- `First allowed action:` states the next concrete action that may happen only
  after this classification.
- `Stop/blocker:` states `None` or the exact blocker that prevents proceeding.

## Gates

- Missing classification evidence blocks branch/worktree setup, delegation,
  validation/QA, implementation, PR handling, review-response routing, merge
  work, release work, and PR-readiness claims.
- Classification must happen before acting on or using the owner decision to
  edit files, set up branches or worktrees, delegate lanes, or route review
  feedback.
- Classification cannot waive root `AGENTS.md`, user stop conditions,
  selected skills, current-head review gates, or packaged `codexy-sentinel`.
- A broad or bundled request must be split into atomic lanes before any
  implementation lane begins unless a maintainer explicitly scopes it as one
  atomic lane before edits.
- If the classified lane touches plugin packaging, skills, validators, MCP,
  LSP, release, or GitHub surfaces, include the matching validation or external
  observation in required evidence.

## Failure Modes

- Starting implementation first and classifying afterward.
- Creating a branch or worktree before deciding whether the lane is
  parent-owned or child-owned.
- Treating review response, merge, validation, release, and implementation as
  one generic task.
- Letting a parent patch a child-owned implementation or review-response lane.
- Reporting a PR-ready handoff without classification evidence in the proof
  bundle.
