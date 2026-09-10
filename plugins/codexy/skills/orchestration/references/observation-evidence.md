# Observation Evidence Boundaries

Use these boundaries when a Watcher reports a material event or an absence from
an observed source. They preserve the distinction between visibility, delivery,
and transport evidence.

A Watcher MUST keep the Worker/app transcript, the parent callback-delivery
receipt, and the Watcher MCP report/queue as separate evidence surfaces. A
terminal-looking Worker result is only an app observation; it does not prove
that a callback reached the parent.

A blank, sparse, or incomplete app transcript—including a completed
`read_thread` turn with `items: []`, no `latestAssistantMessageId`, omitted
fields, partial pages, or truncated output—MUST be recorded as unknown
transcript visibility. It MUST NOT by itself produce a `missing_delivery`
conclusion. The same rule applies to a filtered or partial review-finding
inventory: no matching item is not proof of absence.

Before reporting `missing_delivery`, the Watcher MUST identify the exact Worker
target and callback event or transition key, record the actual app-thread
observation and the separate parent callback-delivery observation, and ensure
the latter is complete enough to establish absence. If the parent-delivery
observation is unavailable, filtered, truncated, or otherwise incomplete, the
Watcher MUST report uncertainty or an unavailable observation rather than
`missing_delivery`. A genuine missing callback remains reportable when the
expected callback and exact identity are known and the complete parent-delivery
surface shows that identity absent. The report remains an untrusted signal; the
Orchestrator MUST validate it against current Worker and callback evidence
before acting.
