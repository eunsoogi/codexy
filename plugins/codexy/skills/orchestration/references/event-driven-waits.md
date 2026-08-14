# Event-Driven Waits

MUST treat asynchronous completion as event waits, not blockers. Parent
orchestrators and child owners MUST use event-driven `wait_threads` with each
target's latest cursor as the default for ordinary child-completion or attention
waits. They MUST reserve heartbeat scheduling for genuinely scheduled monitoring
or when `wait_threads` is unavailable.

After a host transition or `No handler registered` failure, the owner MUST treat
the mismatch as host-transition exposure evidence. It MUST perform one fresh
thread-tool discovery and one host-aware `wait_threads` retry before any
fallback, MUST NOT use unbounded `read_thread`, and MUST record only returned
size/token metadata for a bounded metadata fallback that consumes the current
parent-stage budget.

When an eligible external gate outlives the turn, MUST follow
`references/runtime-heartbeats.md`. Live Sentinel observation MUST be read-only
and event-driven. Generic child and ledger polling remains permitted. Both the
child owner and root orchestrator MUST NOT message, interrupt, replace,
duplicate, follow up with, or poll a live Sentinel. A bounded wait without an
event is a non-terminal `PENDING` observation, and an independently observed
live reviewer is `RUNNING`; neither is a reviewer verdict or fallback-eligible.
The owning lane MUST retain the same reviewer and wait for its natural terminal
result. A live Sentinel MUST report its own terminal `PASS`, `BLOCK`, or
`UNOBSERVABLE` result naturally.
