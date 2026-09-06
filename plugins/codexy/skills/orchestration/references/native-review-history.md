# Native review history recovery

Use this reference when an existing post-PR review result is present in the
Codex host but the durable review-control state lacks a faithful history
representation. This is a source-normalization boundary, not a second ledger,
review authority, admission decision, or merge permission.

## Capture

The caller MUST use the supported Codex host read_thread surface to capture the
complete owner and reviewer page chains. Each raw page MUST retain its thread
identity, page metadata, turns, item IDs, and original UTF-8 text. Continuation
pages are valid only when every intermediate page declares a continuation and
the supplied next page completes the chain. A terminal page MUST declare that no
continuation remains. The normalizer MUST reject truncated, contradictory,
duplicate, or reordered source records.

The owner chain MUST contain exactly one completed collabAgentToolCall
`spawnAgent` or `spawn_agent` item whose sole receiver is the reviewer thread.
Its sender, receiver, prompt, model, and reasoning effort are source facts.
Unrelated spawn items MUST remain in the receipt as raw excluded helpers. A
helper MUST NOT substitute for the reviewer invocation.

The reviewer chain MUST identify completed turns with one final
agentMessage/AgentMessage item and explicit full, delta, or
required_current_head metadata. The reviewed head and terminal result MUST be
explicit and unambiguous. Findings retain their observed text, path, severity,
disposition, and semantic source. When a pathless observation has explicit
semantic metadata, preserve it; when that metadata is absent, preserve the null
path and mark the observation unclassified. The adapter MUST NOT infer a
proof/process kind or invent a file path.

## Projection

The receipt stores the raw pages and final messages beside a derived
history_projection. Existing finding IDs are preserved. Missing IDs are
deterministically derived from the final message and source span and MUST be
marked metadata.derived=true. Source-local sequence and observed completion
timestamps are retained; a global rollout ordinal MUST NOT be fabricated.
Unclassified pathless observations remain unresolved for downstream admission
even if a caller-supplied disposition says otherwise. When every event has a
timestamp, the projection orders events by completion time and rejects ties.

An event-level reviewer model or effort overrides the owner spawn facts only for
that event. If no event-level override is present, the receipt records that the
values came from owner_spawn. The projection preserves full, delta, and
current-head events without assuming a universal event count.

## Current binding and admission

The caller MUST capture a separate current PR snapshot through the authenticated
GitHub readback seam and bind it after normalization. The snapshot MUST bind
repository, PR number, URL, base, head, and capture method. A current head is
not a historical reviewed head. If numeric PR creation and event-completion
times are available, every event MUST complete after PR creation; otherwise
temporal status remains unresolved.

The pure receipt reports authentication=not_attested and result=not_admitted
even when the snapshot contains an authenticated=true field. Only the existing
authenticated producer and transition consumers may admit state. Recovery MUST
preserve an existing direct history and MUST NOT mark findings resolved, waive
quotas, dispatch another review, or authorize completion.
