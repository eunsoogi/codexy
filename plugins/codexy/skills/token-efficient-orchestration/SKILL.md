---
name: token-efficient-orchestration
description: Use during long Codexy orchestration, multi-PR monitoring, review-response loops, or compaction recovery when token use is growing; preserves proof gates while replacing repeated full-context reloads with deltas, ledgers, and bounded polling.
---

# Token-Efficient Orchestration

## Purpose

Keep long Codexy loops small without weakening evidence. Use this skill when a
thread is monitoring several issues or PRs, recovering from compaction, polling
children, routing review feedback, or preparing a handoff that might otherwise
repeat large unchanged artifacts.

This skill is not a shortcut around `$proof-driven-completion`. It changes how
evidence is summarized and refreshed, not which gates are required.

## Required Proof Gates

Never compress away these current facts for an active lane:

- issue and PR numbers,
- branch and worktree path,
- owner boundary and child thread id,
- current head SHA and base SHA,
- check state,
- Codex review state for the current head,
- unresolved review thread ids and whether they are outdated,
- verification commands and results,
- merge readiness or explicit wait/stop condition.

Some lane gates legitimately do not exist yet, especially in issue-only,
pre-PR, parent-owned, or pre-review loops. Record those slots explicitly as
not-created or not-applicable states with a short reason instead of inventing
evidence or stalling the lane. Use those states only when the gate genuinely
does not apply or has not been created yet. For gates that should exist for the
current lane, refresh existing gates directly instead of inferring them from
older context.

## Token Budget Loop

Run this loop before large polling batches, after compaction, and before
handoff:

1. **Inventory once**: list active lanes as one line each with `issue`, `PR`,
   `branch`, `head`, `owner`, and `state`.
2. **Poll by delta**: refresh only surfaces that can change: PR head, checks,
   review threads, Codex review output, and child status. Do not re-read
   unchanged skill bodies, old review text, or full logs unless a changed id or
   SHA requires it.
3. **Promote ids, not prose**: keep exact ids and links, such as PR numbers,
   thread ids, review thread ids, check run names, and SHAs. Summarize bodies
   in one sentence unless the exact wording is the bug.
4. **Demote stale details**: mark old heads, resolved comments, passed reruns,
   and outdated review suggestions as stale or resolved. Do not carry them as
   active obligations.
5. **Carry one next action**: each lane should end with exactly one next action:
   route feedback, wait for review, wait for checks, verify child handoff,
   resolve fixed thread, merge, or stop.

## Delta Poll Shape

Use this compact shape for each lane:

```text
#<issue> / PR #<pr> / <branch>
owner: child thread <id> | worktree <path>
head: <sha> | base: <sha>
delta since last poll: <new head/check/thread/review change or "none">
required gates: checks=<state>; codex-review=<state>; threads=<state>; child=<state>
active obligations: <only current unresolved work>
stale/demoted: <old heads, resolved threads, superseded comments>
next action: <one action>
```

When nothing changed, write `delta since last poll: none` and skip the old
details.

For repeat handoffs, copy `templates/delta-poll.md` and fill only the current
slots. Keep the template output in the thread or handoff; do not attach old
logs or unchanged review bodies unless a current gate points to them.

## Compaction Budget

After compaction, rebuild only the working set:

- active lanes and their latest known SHAs,
- unresolved current-head review thread ids,
- child ownership and stop condition,
- commands already run only when their result still proves a current gate,
- known tool exposure mismatches that affect the next action.

Do not reload old full review bodies, full command output, resolved feedback,
or closed lanes unless a current gate references them.

## Handoff Discipline

For a child handoff or parent status, include:

- `remember`: durable facts needed for the next gate,
- `refresh`: facts that must be re-polled before action,
- `forget`: resolved, outdated, superseded, or irrelevant details,
- `next`: one action with the owner.

Use `forget` for stale context. It means the detail should not drive active
work unless a fresh poll makes it current again.

## Stop Conditions

Stop and refresh rather than summarizing when:

- the head SHA changed,
- a check moved from pending to pass/fail,
- a new Codex review arrived,
- a review thread changed resolved or outdated state,
- child ownership is unclear,
- the next action would merge, resolve a review thread, or claim readiness.

These actions require current authoritative evidence, not a cached summary.
