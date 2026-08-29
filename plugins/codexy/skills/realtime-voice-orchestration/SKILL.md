---
name: realtime-voice-orchestration
description: Use when a user explicitly requests a realtime voice interaction that must route a task or status request to an authoritative Codex project owner and summarize verified progress without taking over orchestration.
---

# Realtime Voice Orchestration

MUST use this skill only when the user explicitly asks for voice interaction and
`$orchestration` selects this experimental adapter. `$orchestration` remains
authoritative for ownership, dispatch, child coordination, evidence, handoffs,
thread state, and completion. Disposition: `KEEP + SIMPLIFY`; default routing:
`DEFER`.

## Resolve and route

- MUST resolve conversational references against available context and
  authoritative active-thread state; MUST NOT assume the visible voice thread
  owns the project.
- MUST route to the known parent only, or directly to exactly one relevant
  standalone owner when no parent exists. With no active owner, respond
  conversationally or offer to start a task.
- If multiple owners remain plausible, MUST ask exactly one concise
  clarification and wait; dispatch count is zero until the user selects one.
- MUST send exactly one request to the selected owner. If routing is ambiguous
  or fails, report that it was unconfirmed and give the next safe action; MUST
  NOT retry or duplicate dispatch.
- The parent MUST remain the sole coordinator. The voice layer MUST NOT invent a
  parent, route to an unrelated thread, or steer, poll, or decide for a parent's
  children.
- MUST capture current-screen context only when ambiguity is material and the
  surface is available. Otherwise report unavailable rather than inferring or
  replacing the context; MUST NOT patch the native host.

## Confirmed state and interruption

- MUST speak only after the selected owner returns a confirmed authoritative
  result. Summarize verified aggregate state; omit identifiers, transcripts, and
  internal events.
- MUST distinguish `in progress`, terminal `success`, `failure`, `cancellation`,
  and `blocked`. Include a confirmed reason where available; MUST NOT call
  progress complete.
- When the user interrupts, MUST yield promptly while preserving the selected
  owner, durable work, and latest confirmed route. Interruption alone MUST NOT
  cancel or redispatch. Cancellation or retry requires explicit owner support
  and current authoritative state. MUST resume with the newest verified state.
- MUST monitor only the selected parent or standalone owner through bounded,
  event-driven observations. MUST NOT poll unboundedly, guess from silence, or
  monitor a child directly. Without a terminal event, report `in progress` and
  the next observable milestone.
- On host transition or unavailable thread capability, MUST preserve the owner
  and report the limitation; MUST NOT substitute another thread or guess status.

## Static contract projection

- V1 known parent + confirmed success: parent only, one dispatch, report success;
  MUST NOT contact a child.
- V2 one standalone owner + confirmed failure: route once, report failure and
  its confirmed reason; MUST NOT invent a parent or relabel failure as blocked.
- V3 two plausible owners: one clarification, zero dispatch; report `blocked`
  on no selection rather than guessing.
- V4 interruption + confirmed cancellation: yield, MUST NOT cancel or duplicate;
  keep the owner and report cancellation after the result.
- V5 selected owner still running: report `in progress` and the next milestone;
  MUST NOT make a terminal claim.
- V6 unavailable authenticated context: report `blocked` and the next safe
  action; MUST NOT guess or substitute another task.
- VN1 typed non-voice input: remain on normal orchestration and select zero
  voice-adapter bytes.
- VN2 child steering while a parent exists: route to the parent or refuse direct
  steering; child dispatch count remains zero.

## Boundaries

This skill MUST NOT replace `$orchestration`, become an owner, create a second
parent, change child ownership, steer a child, guess state, blindly retry,
export transcripts, or execute release actions. Local verification, PR/merge,
and external release phases MUST remain separate; never imply publication. For
non-voice work, MUST continue on the normal task-selected orchestration route.
