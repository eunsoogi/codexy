---
name: orchestration
description: MUST use first to classify a Codex task and select only the references authorized by the context-tier contract.
---

# Orchestration

This is the always-on classifier and reference router for general Codex work.
The plugin-invoking task MUST remain the root/orchestrator; a child worktree
owns only its assigned atomic lane. Read the contract before classifying.

## Sole authority

`references/context-tiers.json` is the sole routing and safety authority. MUST
use its routing, profile, retained-field, authority, budget, ordering,
forwarding, and forbidden-context data as-is. MUST NOT copy its task-class
vocabulary or route table, and MUST NOT define a competing authority.

If the contract is missing, malformed, incomplete, or disagrees with the
packaged copy, MUST fail closed through its `fallback_authority` and preserve
every safety invariant. A budget, cache, model route, or profile MUST NOT
authorize an omission that the contract forbids.

## Entry procedure

1. Read `context-tiers.json` and classify the request with its closed task set.
2. Select the manifest's `task_reference_routes` entry for the workflow, then
   append each selected `surface_reference_routes` entry and any applicable
   `risk_reference_routes` entry in order, deduplicating while preserving order.
   For an ordinary structured classification, this is the same task-plus-
   surface route produced by the validator. Load only that union; the isolation
   invariant is `unrelated references are not loaded`. Resolve each selected
   authority through the contract's `authorities` map; its packaged path is the
   only matching-reference indirection.
3. For unknown/incomplete or security-, permission-, release-, or other
   risk-sensitive classification, use `fallback_reference_route` plus the
   selected `risk_reference_routes` entries; do not union ordinary task or
   surface authorities into an unsafe route. Retain all always-on fields and do
   not authorize action from missing proof.
4. Keep `selected_references`, typed omissions, context forwarding, stable and
   volatile identities, and the one next action exactly as the contract
   specifies. MUST NOT forward a full conversation, tool body, or agent tree.
5. Read the selected references before acting. They own detailed goals, plans,
   child routing, budgets, reviews, waits, handoffs, and verification; this
   entrypoint MUST NOT restate or override them.

## Public extension boundary

The `$orchestration` contract remains public. When selected work crosses GitHub
or developer tooling, invoke the installed `codexy-github` / `$git-workflow` and
`codexy-devtools` / `$developer-tools` contracts. MUST NOT derive private paths
or require repository-only validation commands in this installed skill. If an
extension or required tool is unavailable, MUST fail closed and report the
unavailable surface.

When the selected route includes `public_extension_contracts`, read its
manifest-resolved authority and apply only the matching contract inside it
(`issue-intake receipt`, `child-lane-ownership`, or `completion-handoff`). MUST
NOT load or reproduce the other contracts.

## Controlled proof boundary

Controlled routing acceptance MUST use the frozen
`routing-evaluation-corpus.json`, `routing-evaluation-results.schema.json`, and
`routing-evaluation-results.json`. Preserve zero `P0/P1` misses, reject
incomplete or unavailable evidence, and never synthesize measurements. Model
selection, reviewer budgets, session-audit behavior, handoff schema, GitHub
behavior, and publication behavior remain owned by their authorities.
