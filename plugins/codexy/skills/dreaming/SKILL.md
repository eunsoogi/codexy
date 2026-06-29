---
name: dreaming
description: Use when a Codexy lane resumes after context compaction, inherited summaries feel stale or overfull, resolved work keeps reappearing as active, or an agent must separate durable facts, active fixes, and stale details before continuing.
---

# Dreaming

## Purpose

Run a short memory hygiene pass before continuing after compaction, long
handoffs, or noisy multi-PR orchestration. The goal is to keep the next action
anchored in current evidence instead of in whatever the compacted summary made
most prominent.

This skill is a thinking and handoff discipline. It does not write durable
memory by itself, close review threads, update branches, or replace the
workflow skill that owns the lane.

## Use When

- A Codexy thread resumes from compacted context, a summarized handoff, or a
  stale continuation.
- Resolved review feedback, old check failures, old branch heads, or duplicate
  lane notes keep appearing as active work.
- The next agent must decide what to remember, what to fix, and what to forget
  or demote before acting.
- A compact handoff needs to preserve the current stop condition without
  carrying stale obligations forward.

Do not use this as a substitute for fresh git, GitHub, validator, LSP,
codegraph, issue, or PR evidence. Dreaming classifies evidence; it does not
create evidence.

## Core Rule

Separate every carried fact into exactly one bucket:

| Bucket | Keep when | Required evidence |
| --- | --- | --- |
| Remember | It is durable project policy, issue scope, owner boundary, exact IDs, current refs, or a stop condition the next agent must preserve. | Current instruction, issue, PR, git, or tool output. |
| Fix | It is an unresolved obligation that still needs action on the current lane. | Current failing check, unresolved review thread, open issue, dirty worktree, or explicit maintainer request. |
| Forget or demote | It is resolved, superseded, stale, duplicated, only historical, or useful as background but not action-driving work. | Current state proves it is no longer active, or it lacks current evidence. |

Do not carry an item as `Fix` only because it appears in a summary. Verify it
against the authoritative surface first.

## Dream Pass

1. Re-read the governing instruction source for the lane.
2. Capture current anchors: `pwd`, branch, `HEAD`, base ref, issue, PR, owner,
   and stop condition.
3. Compare inherited claims with current evidence.
4. Move each claim into `Remember`, `Fix`, or `Forget or demote`, with one
   evidence note per active `Fix`.
5. Continue only from the `Fix` bucket, the `Remember` constraints, and the
   current stop condition.

For Codexy GitHub lanes, current evidence usually means `git status`, `git log
--graph`, PR head SHA, checks, review threads, latest Codex review output, and
child owner state.

## Compact Handoff Shape

Use this shape when writing or repairing a compacted continuation summary:

```text
Dream pass:
Current anchors:
- Worktree:
- Branch:
- HEAD:
- Base:
- Issue/PR:
- Owner:
- Stop condition:

Remember:
- Durable policy, scope, owner boundaries, exact IDs, and current refs.

Fix:
- Current unresolved obligations only, each with current evidence and next
  action.

Forget or demote:
- Resolved review feedback, stale SHAs, old branch state, duplicate lanes,
  outdated checks, superseded summaries, and historical notes that must not
  drive the next action.

Next action:
- The single next action allowed by the current owner boundary and stop condition.
```

## Review And PR Hygiene

- Treat resolved review threads as `Forget or demote`, unless a current review
  reopens the same concern.
- Treat outdated-but-fixed threads as history after they are resolved in
  GitHub and current-head evidence proves the fix.
- Treat old branch heads, old check failures, old review output, old CI state,
  and old PR mergeability as stale when a newer commit exists.
- Keep active only the latest unresolved review threads, pending checks,
  dirty worktree changes, or maintainer requests that match the current head.
- If a summary says something was fixed, verify the current PR thread state
  before removing it from `Fix`.
- If a handoff names a branch, SHA, PR state, or review result that does not
  match the current surface, demote the inherited claim and continue from the
  refreshed surface.

## Common Mistakes

| Mistake | Correction |
| --- | --- |
| Keeping every compacted bullet as active work. | Reclassify by current evidence. |
| Losing the stop condition. | Put it in `Current anchors` before any next action. |
| Treating resolved review feedback as still open. | Demote it after current GitHub evidence confirms resolution. |
| Treating stale checks or old review output as current. | Compare timestamps and head SHAs before acting. |
| Forgetting ownership after compaction. | Preserve parent/child owner boundary as a `Remember` item. |
| Writing a continuation from memory alone. | Refresh current anchors first, then classify. |

## Stop Conditions

Stop and refresh evidence before editing when:

- the current branch or head SHA is unknown,
- inherited summary claims conflict with GitHub or git state,
- an item cannot be classified into one bucket,
- owner boundary or stop condition is missing,
- a resolved item appears to require action but no current surface proves it.
