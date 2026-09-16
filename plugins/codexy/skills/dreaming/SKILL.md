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

## Active plan recovery

When the task already provides one active plan or an exact plan path, MUST read
only that plan as recovery context after refreshing live state. MUST NOT discover
or select another plan.

- MUST treat the plan as an auxiliary record, not authority over the native goal,
  current owner, worktree, Git/GitHub state, review, proof, or completion.
- MUST compare its objective, current evidence, preserved behavior, scope,
  dependency inputs, decisions, assumptions, unknowns, and stop condition with
  the refreshed live state. Keep confirmed facts in `Remember`.
- When a plan claim conflicts with current evidence, MUST keep the live fact in
  `Remember` or `Fix`, put the stale plan claim in `Forget or demote`, and
  explain the difference in the refreshed output. MUST NOT silently reconcile
  the conflict or use the plan to invent an owner, completion, next action, or
  permission.
- MUST NOT update, overwrite, mark complete, reopen, or change the plan or its
  storage/exclusion state. A request to create or update a plan goes to
  `$planning`; execution, ownership, GitHub, review, and completion decisions
  remain with their existing authorities.

## Remember, Fix, Forget

MUST place each carried claim in exactly one bucket:

| Bucket           | Keep only                                                     |
| ---------------- | ------------------------------------------------------------- |
| Remember         | Current policy, scope, refs, owner, and stop condition.       |
| Fix              | A current unresolved obligation with evidence.                |
| Forget or demote | Resolved, stale, superseded, duplicated, or unproved history. |

MUST continue only from Remember constraints and Fix obligations. MUST emit one
next action allowed by current owner and stop condition; MUST NOT invent one. If
a carried claim needs reclassification, MUST show it in its bucket. MUST return
byte-identical `NO_CHANGE` only when current surfaces are clean and there is
nothing to reclassify; MUST NOT create a report or other artifact.

## Implementation sanity

When the current `Fix` is an implementation obligation, MUST run one bounded
sanity pass before choosing the next action. When the current `Fix` is read-only
or otherwise non-implementation recovery, MUST NOT run it.

MUST use only the current diff, governing scope, and meaningful tests to ask
whether:

1. the change repairs the evidenced structural cause or merely stacks another
   edge-case exception;
2. each relevant test exercises realistic required behavior and a regression,
   rather than only the current implementation shape or generated wording; and
3. a materially simpler design satisfies the same requirements, invariants, and
   coverage.

MUST report a patch-stack, test-for-test, or avoidable-complexity concern only
when current evidence supports it. MUST NOT report an unsupported concern. MUST
put that concern in `Fix` and MUST make the single next action the smallest
structural correction, simpler-design check, or behavior/regression-proof
correction; MUST NOT choose a different follow-up action. When the
implementation is structurally sound, MUST continue under the existing output
contract without ceremonial extra output.

This pass is bounded and advisory. It MUST NOT become a general reviewer,
prescribe an architecture, weaken requirements, delete legitimate edge-case
coverage, narrow scope, mutate state, or replace orchestration, GitHub,
reviewer, or completion authority.

## Capsule Compatibility

The v1 compaction, fresh-child, and parent-handoff invocation contract remains
supported. Installed `scripts/resumable-context-capsule.sh` or `.cmd` launchers
MUST validate a capsule through the native bridge with a separate trusted live
authority document before it is consumed.

Before changing or removing a schema, launcher, or resolver, MUST inventory its
direct, dynamic, package, and public consumers. If any consumer cannot be
classified, stop with `BLOCKED_CONSUMER_UNKNOWN` and preserve the invocation.

## Output

MUST return only the refreshed current anchors, Remember, Fix, Forget or demote,
and one next action. Fixes MUST cite current evidence. Dreaming MUST NOT close
threads, edit branches, change owners, direct children, reset review counts,
write memory, or replace orchestration, GitHub, reviewer, or completion
authority.
