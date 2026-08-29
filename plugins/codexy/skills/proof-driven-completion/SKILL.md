---
name: proof-driven-completion
description: MUST use before claiming work is done, handing off, opening or merging a PR, closing an issue, reporting success, or completing a goal for code, docs, workflow, UI, plugin, marketplace, or release tasks.
---

# Proof-Driven Completion

## Purpose

Completion is a current-state claim. MUST map every requirement to evidence that
proves it on the authoritative surface, and MUST stop when proof is absent,
stale, too weak, or contradictory.

## Audit

1. MUST restate the requested outcome and make a finite list only from its
   explicit requirements, named files, commands, external states, and
   deliverables.
2. For each item, MUST name the evidence that would prove it. MUST use file
   content or diff for files, parsers for structured data, tests for executable
   behavior, and the authentic CLI, GitHub, browser, desktop, plugin,
   marketplace, or release surface for externally observable claims.
3. MUST inspect the current authoritative state. Current head and current
   external state MUST win over memory, intent, plans, and output from older
   revisions.
4. `proved` means current evidence matches every stated requirement; MUST NOT
   invent extra gates. `contradicted` conflicts; `incomplete` is partial or
   stale; `too weak` uses the wrong scope or surface; `missing` is absent or
   unrun proof. A missing required proof makes completion `missing`, not
   `incomplete`.
5. MUST continue until every required item is proved. Otherwise MUST stop the
   completion claim, name the unmet item, and identify one concrete next action.

## Invariants

- A unit test alone MUST NOT prove GitHub, CLI, browser, desktop, plugin,
  marketplace, publication, or release behavior; MUST drive the matching surface.
- An open PR, green CI, merge, publication, and milestone closure are distinct
  states and MUST NOT substitute for one another.
- Evidence from an older head MUST NOT prove the current head. An unresolved
  external gate keeps the corresponding requirement incomplete.
- For governed files, MUST use the canonical touched-LOC producer. Every file
  MUST be at or below 250 physical lines, and blank-line deletion or collapsed
  readable content MUST NOT count as structural remediation.
- Missing goal, plan, tool, or multi-agent receipts alone MUST NOT invalidate
  otherwise current and complete outcome evidence. Those producers own their own
  process contracts.
- GitHub, owner, reviewer, CI, release, and external-state producers own their
  state machines. Consume their current evidence without restating or replacing
  their authority here.

## Completion Report

MUST report the outcome, changed surfaces, verification and authentic-surface
observations, skipped checks with reasons, unresolved gates, residual risks, and
the next action. MUST NOT describe incomplete work as done.

Completing a finite execution goal proves only that named phase; it MUST NOT
claim the issue, PR, merge, release, or external gate is complete unless each is
separately proved.
