---
name: spec-driven-development
description: Use when a task starts from a PRD, issue, acceptance criteria, design brief, API contract, user story, or ambiguous feature request that needs implementation discipline before editing.
---

# Spec-Driven Development

## Purpose

Treat the spec as the implementation contract. Convert intent into observable
claims, prove the claims with targeted evidence, and keep the branch scoped to
one issue-sized outcome.

## Workflow

1. Locate governing sources:
   - latest user request,
   - GitHub issue or maintainer-provided scope,
   - PRD, design brief, API contract, or acceptance criteria,
   - `AGENTS.md` and nested instructions,
   - relevant project or plugin skills.
2. Extract requirements:
   - hard requirements,
   - preferences,
   - out-of-scope items,
   - assumptions that need verification,
   - user-visible success criteria.
3. Reduce to an atomic outcome:
   - split unrelated behavior into follow-up issues,
   - name the owning branch or worktree,
   - avoid bundling cleanup unless it is required to prove the spec.
4. Define proofs before implementation:
   - one proof for the happy path,
   - one proof for the riskiest boundary or edge case,
   - one regression proof for behavior that must not change,
   - one external-surface proof when the spec affects CLI, GitHub, browser,
     desktop, plugin, marketplace, or repository settings.
5. Implement only spec-backed changes.
6. Re-run proofs and map each changed file back to a requirement.
7. Before PR or merge, audit whether every explicit requirement has current
   evidence.

## Required Output

```text
Spec source:
Atomic outcome:
In scope:
Out of scope:
Success criteria:
Proof plan:
Open questions:
```

## Gates

- Do not edit production files until the atomic outcome and proof plan are
  clear.
- Do not widen scope because a nearby improvement is tempting.
- Do not open a PR until every changed file maps to the spec or a named support
  requirement.
- Do not merge until review feedback and spec evidence both pass on the latest
  head.

## Evidence Rules

- A test proves only the behavior it asserts.
- A parser check proves syntax or schema, not user-visible behavior.
- A screenshot or UI observation proves only the visible state it captures.
- A GitHub API response proves only the repository state at that timestamp.
- If evidence is stale after a new commit, rerun it or label it stale.

## Failure Modes

- Treating broad prose as permission for a broad branch.
- Replacing acceptance criteria with implementation details.
- Claiming completion from green tests while the requested external surface was
  never driven.
- Forgetting to record out-of-scope items, then accidentally implementing them.
