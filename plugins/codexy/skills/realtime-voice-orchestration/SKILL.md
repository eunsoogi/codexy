---
name: realtime-voice-orchestration
description: Use when a realtime voice conversation must route a task or status request to an authoritative Codex project owner and summarize verified progress without taking over orchestration.
---

# Realtime Voice Orchestration

Use this skill only after `$orchestration` has classified the request as a
realtime voice interaction. It is a voice-specific routing and presentation
adapter around the normal orchestration contract. Normal orchestration remains
the canonical authority for ownership, dispatch, child coordination, evidence,
handoffs, and thread state.

## Canonical flow

The supported ownership flow is parent-or-standalone-owner:

- Parent-owned:
  `voice input -> owning orchestrator/parent -> parent-managed child coordination -> parent result -> voice summary`
- Standalone-owned:
  `voice input -> exactly one relevant standalone active project owner -> owner result -> voice summary`

- MUST route a project request to its owning parent/orchestrator when one is
  known, or directly to exactly one relevant standalone active project owner
  when no parent exists. The voice layer MUST NOT steer, poll, or decide for a
  parent's children directly, invent an orchestrator for a standalone owner, or
  route to an unrelated thread.
- MUST NOT assume that the visible voice thread owns the project. A voice thread
  may be separate from the project parent or a standalone work owner.
- MAY inspect enough authoritative parent state to resolve context and report
  status, but MUST NOT become a parallel project manager.
- MUST treat native thread-tool or current-screen capability gaps, including the
  host dependency represented by #611, as unavailable context. State the limit;
  MUST NOT patch the native host or invent missing state.

## Resolve the project context

MUST use the user's conversational references, the current visible context when
it is available, and authoritative active-thread state. Resolve the route with
this closed decision table:

| Observed context                                             | Route                                             | Voice-layer boundary                                            |
| ------------------------------------------------------------ | ------------------------------------------------- | --------------------------------------------------------------- |
| A clear owning orchestrator/parent exists                    | Route to that parent only                         | MUST NOT steer, poll, or decide for its children                |
| Exactly one relevant standalone active project thread exists | Route directly to that thread                     | MUST NOT invent an orchestrator or add child coordination       |
| More than one project workflow remains plausible             | Ask one concise clarification                     | MUST NOT choose by guess or inspect unrelated projects          |
| No active work owner exists                                  | Respond conversationally or offer to start a task | MUST NOT route to unrelated threads or manage orphaned children |

- MUST capture current-screen context only when a visible reference is
  materially ambiguous and that surface is available. If it is unavailable, say
  so instead of inferring what the user meant.
- MUST distinguish parent ownership from child execution when that distinction
  helps the user understand status. The parent remains the sole coordinator.
- MUST use at most one concise clarification for a materially ambiguous project
  reference, then wait for the user's answer before routing.

## Dispatch and state reporting

- MUST send one route request to the selected owner. MUST emit a voice-facing
  state update only after the relevant dispatch returns a confirmed
  authoritative result.
- If dispatch is ambiguous or fails, report that it was not confirmed and the
  next safe action. MUST NOT blindly retry, duplicate dispatch, or claim that
  work started.
- Summarize verified aggregate state, not raw logs or tool payloads. Omit opaque
  thread identifiers, transcripts, and internal event ids from spoken output.
- MUST distinguish `in progress`, terminal `success`, `failure`, `cancellation`,
  and `blocked` states. MUST NOT describe in-progress work as complete.
- Keep local verification, PR/merge, and externally state-changing release
  phases separate. MUST NOT imply that a public release, tag, or asset
  publication happened locally.

MUST use short, plain-language updates that answer what is happening, what has
been verified, and what remains. MUST keep machine-readable evidence in its own
channel. When speaking Korean, use natural Korean rather than translating
workflow nouns literally.

## Interruption-first behavior

- When the user interrupts, yield or stop the current spoken summary promptly.
- Preserve the selected owner, durable project work, and the latest confirmed
  route. An interruption of speech is not permission to cancel project work.
- MUST NOT dispatch the same request again because speech was interrupted.
  Cancellation or retry requires an explicit, owner-supported operation and
  current authoritative state.
- MUST resume with the newest verified state only; MUST NOT continue a stale
  summary or replay raw context.

## Bounded monitoring

- Monitor the selected parent, or the single selected standalone owner, through
  authoritative thread state and bounded/event-driven observations.
- MUST NOT leave the user waiting on an unbounded poll, guess from silence, or
  monitor a child directly when a parent owns that child.
- If no terminal event is available, MUST say that the work is still in progress
  and identify the next observable milestone without claiming completion.
- On a host transition or unavailable thread tool, preserve the owner and report
  the unavailable capability. MUST NOT substitute an unrelated thread, local
  host patch, or guessed status.

## Scope boundary

This skill MUST NOT replace `$orchestration`, create a second parent, change
child ownership, export transcripts, or execute public release actions. For
non-voice work, MUST continue using the normal task-selected orchestration
route.
