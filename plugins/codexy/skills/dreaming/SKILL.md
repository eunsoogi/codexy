---
name: dreaming
description: MUST use when a Codexy lane resumes after context compaction, inherited summaries feel stale or overfull, resolved work keeps reappearing as active, or an agent MUST separate durable facts, active fixes, and stale details before continuing.
---

# Dreaming

## Purpose

MUST run a short memory hygiene pass before continuing after compaction, long
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
- The next agent MUST decide what to remember, what to fix, and what to forget
  or demote before acting.
- A compact handoff needs to preserve the current stop condition without
  carrying stale obligations forward.

MUST NOT use this as a substitute for fresh git, GitHub, validator, LSP,
codegraph, issue, or PR evidence. Dreaming classifies evidence only; it creates
no evidence.

## Core Rule

MUST separate every carried fact into exactly one bucket:

| Bucket | Retention condition | Required evidence |
| --- | --- | --- |
| Remember | It is durable project policy, issue scope, owner boundary, exact IDs, current refs, or a stop condition the next agent MUST preserve. | Current instruction, issue, PR, git, or tool output. |
| Fix | It is an unresolved obligation that still needs action on the current lane. | Current failing check, unresolved review thread, open issue, dirty worktree, or explicit maintainer request. |
| Forget or demote | It is resolved, superseded, stale, duplicated, only historical, or useful as background but not action-driving work. | Current state proves it is no longer active, or it lacks current evidence. |

MUST NOT carry an item as `Fix` only because it appears in a summary. MUST verify it
against the authoritative surface first.

## Dream Pass

1. MUST re-read the governing instruction source for the lane.
2. MUST capture current anchors: `pwd`, branch, `HEAD`, base ref, issue, PR, owner,
   and stop condition.
3. MUST compare inherited claims with current evidence.
4. MUST move each claim into `Remember`, `Fix`, or `Forget or demote`, with one
   evidence note per active `Fix`.
5. MUST continue only from the `Fix` bucket, the `Remember` constraints, and the
   current stop condition.

For Codexy GitHub lanes, current evidence usually means `git status`, `git log
--graph`, PR head SHA, checks, review threads, latest Codex review output, and
child owner state.

## Active Child Thread Ledger

For orchestration, dogfooding, review-response, or multi-lane handoffs, dreaming
MUST preserve a durable active/waiting child thread ledger instead of reducing
child work to a one-time summary. The dream pass MUST collect the full
in-progress/waiting child thread list from current issue, PR, thread, worktree,
and handoff evidence before deciding whether new child work is needed.

Each active or waiting Codex app child thread entry MUST include issue/PR, thread
id, status, owner state, blocker, latest evidence, and next action. Blocked or
rate-limited child lanes MUST stay in the ledger with the current blocker and
next recheck action until current evidence proves they are complete, reassigned,
or intentionally abandoned by a maintainer.

Completed child lanes MUST be removed from the ledger after current evidence
proves completion, and the completed child thread MUST be archived/deleted where
supported by the available tool surface. If archive/delete tooling is
unavailable, the dream pass MUST record that unavailable-tool evidence and MUST
still remove the completed lane from the active/waiting ledger.

## Compact Handoff Shape

MUST use this shape when writing or repairing a compacted continuation summary:

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
- Active/waiting child thread ledger entries, each with issue/PR, thread id,
  status, owner state, blocker, latest evidence, and next action.

Fix:
- Current unresolved obligations only, each with current evidence and next
  action.

Forget or demote:
- Resolved review feedback, stale SHAs, old branch state, duplicate lanes,
  outdated checks, superseded summaries, and historical notes that MUST NOT
  drive the next action.
- Completed child lanes removed from the ledger after archive/delete or
  unavailable archive/delete evidence.

Next action:
- The single next action allowed by the current owner boundary and stop condition.
```

## Review And PR Hygiene

- MUST treat resolved review threads as `Forget or demote`, unless a current review
  reopens the same concern.
- MUST treat outdated-but-fixed threads as history after they are resolved in
  GitHub and current-head evidence proves the fix.
- MUST treat old branch heads, old check failures, old review output, old CI state,
  and old PR mergeability as stale when a newer commit exists.
- MUST keep active only the latest unresolved review threads, pending checks,
  dirty worktree changes, or maintainer requests that match the current head.
- If a summary says something was fixed, MUST verify the current PR thread state
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
| Forgetting ownership after compaction. | MUST preserve parent/child owner boundary as a `Remember` item. |
| Writing a continuation from memory alone. | Refresh current anchors first, then classify. |

## Stop Conditions

MUST stop and refresh evidence before editing when:

- the current branch or head SHA is unknown,
- inherited summary claims conflict with GitHub or git state,
- an item does not fit exactly one bucket,
- owner boundary or stop condition is missing,
- a resolved item appears actionable but no current surface proves it.
