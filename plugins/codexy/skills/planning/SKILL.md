---
name: planning
description: Use when a user asks to create, update, or locally save an actionable project plan, including $planning, rather than a status summary or simple question.
---

# Planning

## Trigger and boundary

MUST use this skill for an actionable project plan, a plan update based on new
evidence, or a request to keep such a plan locally. `$planning` invokes the same
workflow.

MUST NOT select this skill for a status summary, a simple question, or an
execution request that has no planning work. A planning request MUST NOT by
itself create a GitHub issue, assign work, start execution, deploy anything, or
replace an existing owner, goal, review, or verification authority.

The plan is an auxiliary document. MUST preserve the native goal contract when
one is required, and MUST use `$orchestration` for ownership, issue, branch,
worktree, GitHub, review, or goal decisions. A plan MUST NOT become a text
replacement for a native goal or a new receipt contract.

For a goal-controlled task, MUST use the actual native goal object and MUST
verify its current state and transitions through the existing goal authority.
MUST NOT replace it with a text goal or bypass its API. If a plan includes
Workers, event waits, or a Watcher, it MUST preserve the existing read-only
Watcher observation, deduplication, parent judgment, and parent-await route; the
parent MUST NOT wait on Workers directly. It MUST preserve the assigned model,
reasoning, host, and permission policy, and MUST report a missing capability; it
MUST NOT silently fall back.

## Prepare the plan

1. MUST determine whether the request asks to save or update a plan, or only to
   show output. Explicit read-only, output-only, and shared-plan choices MUST be
   honored.
2. MUST use the latest available sources that matter: the current request,
   current task or issue, repository instructions, relevant files, and named
   external state. Memory or history MAY help when requested or useful, but MUST
   NOT be a mandatory collection step for every plan.
3. MUST separate the following facts in the plan: objective, current evidence,
   behavior to preserve, scope and exclusions, decisions, assumptions, and
   unknowns. MUST mark an unknown instead of filling it with an inference.
4. For a small request, MUST provide a short plan. MUST NOT force Waves, GitHub
   issues, a separate review, or other ceremony unless the request or current
   authority requires it.

The agent MAY use the [plan template](references/plan-template.md) when a
reusable plan document is useful. The plan's wording MUST use `MUST` for
mandatory work and `MUST NOT` for prohibitions when it gives instructions to an
agent.

## Work-item contract

Every non-trivial work item MUST be understandable without the prior
conversation or a parent plan. It MUST include:

- background and the concrete change;
- allowed paths or other exact scope;
- dependency inputs, naming the needed artifact or contract in addition to any
  dependency ID;
- completion conditions and the verification method;
- explicit exclusions; and
- stop or report conditions, including the owner of the next decision.

MUST NOT hide a shared contract or file conflict inside an item. MUST state
which artifact is produced, who consumes it, and what MUST be true before the
item can start.

## Order and parallelism

MUST map path overlaps, shared contracts, generated inputs, and dependency
outputs before choosing an order. MUST put prerequisite work before its
consumers. MUST propose parallel work only when the items have disjoint writes,
independent inputs, and no shared contract that requires serialized judgment.
Otherwise, MUST keep the order explicit and MUST explain the dependency.

## Save and update

When saving or updating is requested or implied, MUST use the
[local storage rules](references/storage.md). MUST select a destination in this
order: the user's exact path, one existing active plan for the same topic, then
the default path. MUST treat a path conflict or more than one matching active
plan as the only planning ambiguity that requires a confirmation; MUST preserve
the plan output and MUST report the conflict while waiting.

Before an update, MUST read the selected file and MUST identify its topic and
state. MUST preserve unrelated files, user-authored content, shared-plan status,
and a completed or archived state. MUST NOT untrack a tracked plan. MUST update
only the same-topic plan fields that the request covers, and MUST NOT overwrite
user-authored lines. If a user edit conflicts with the update or the file has no
safe same-topic boundary, MUST preserve the original, MUST NOT overwrite it, and
MUST report the limitation.

A read-only or output-only request MUST NOT create `.plans`, change an exclude
file, or write a plan. If exclusion cannot be applied, MUST report that fact and
MUST continue with the permitted plan output or save; MUST NOT claim that the
file is ignored.

## Report

MUST return the plan, its selected sources, and the selected path or
`not saved`. MUST distinguish current evidence from assumptions and unresolved
questions. MUST report whether a file was created, updated, preserved, or left
unchanged. MUST keep issue creation, execution, deployment, native goal state,
verification, review, and release as separate outcomes owned by their existing
authorities.

MUST NOT add an MCP server, planner agent, execution database, personal history,
new model policy, or structured receipt requirement to make planning work.
