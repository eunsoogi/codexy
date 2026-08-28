---
name: dreaming
description: MUST use when an active Codex task resumes after context compaction, inherited summaries feel stale or overfull, resolved work keeps reappearing as active, or an agent MUST separate durable facts, active fixes, and stale details before continuing.
---

# Dreaming

MUST run a short recovery pass after compaction or a noisy handoff. It restores
current constraints without creating another ledger, authority, or durable
memory. It MUST NOT mutate task, Git, GitHub, review, owner, or memory state.

## Refresh Current State

1. MUST re-read the governing instruction and current task or issue scope.
2. MUST refresh the worktree, branch, HEAD, base, issue or PR, checks, review
   threads, owner, and stop condition from their authoritative surfaces.
3. MUST compare every inherited claim with that current evidence. A summary or
   memory item is context, never current-state proof by itself.
4. If the current head, owner, stop condition, or conflicting state cannot be
   resolved, stop with `BLOCKED_AUTHORITY_REGRESSION`; MUST NOT infer it.

Current authoritative task/Git/GitHub state wins over inherited summaries and
memory. Resolved feedback and superseded checks stay resolved; a stale head is
demoted, while a current exact-head failure remains active.

## Remember, Fix, Forget

MUST place each carried claim in exactly one bucket:

| Bucket | Keep only |
| --- | --- |
| Remember | Current policy, scope, refs, owner, and stop condition. |
| Fix | A current unresolved obligation with evidence. |
| Forget or demote | Resolved, stale, superseded, duplicated, or unproved history. |

MUST continue only from Remember constraints and Fix obligations. Emit one next
action allowed by the current owner and stop condition; MUST NOT invent one.
If a carried claim needs reclassification, show it in its bucket. Return
byte-identical `NO_CHANGE` only when current surfaces are clean and there is
nothing to reclassify; create no report or other artifact.

## Capsule Compatibility

The v1 compaction, fresh-child, and parent-handoff invocation contract remains
supported. Installed `scripts/resumable-context-capsule.sh` or `.cmd` launchers
MUST validate a capsule through the native bridge with a separate trusted live
authority document before it is consumed.

Before changing or removing a schema, launcher, or resolver, MUST inventory its
direct, dynamic, package, and public consumers. If any consumer cannot be
classified, stop with `BLOCKED_CONSUMER_UNKNOWN` and preserve the invocation.

## Output

Return only the refreshed current anchors, Remember, Fix, Forget or demote, and
one next action. Every Fix MUST cite current evidence. Dreaming MUST NOT close
threads, edit branches, change owners, direct children, reset review counts,
write memory, or replace orchestration, GitHub, reviewer, or completion
authority.
