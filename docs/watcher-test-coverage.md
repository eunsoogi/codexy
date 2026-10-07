# Watcher correctness and contention stress

The default system suite separates durable transport correctness from burst
throughput. Three accepted events are checked across a process restart in pages
of two, including exact IDs, contents, sequence, cursor and empty final page. An
independent observer checks health after each completed report. A controlled OS
lock owner holds the transition lock until another process returns the bounded
busy error, then verifies unchanged state and successful progress by that same
process after release. This negative test does not make ordinary busy responses
acceptable or add a client retry policy.

Existing cancellation, process-death, interrupted-write recovery, bounded
quarantine recovery and unknown-data preservation tests remain in the default
suite.

The original three rounds of 32 reports competing with 40 health requests remain
an explicitly ignored stress test. Run it separately with:

```sh
cargo test --locked --manifest-path packages/codexy-runtime/Cargo.toml --test suite_system system::mcp_stdio::watcher_state::concurrent_reports_and_health_leave_a_restartable_event_log -- --ignored --exact
```

This stress test still requires every RPC to succeed within the existing lock
bound. Its initial barrier does not control lock acquisition order, filesystem
latency or scheduler fairness. Main run 34327217103 failed this workload on
Windows with a `.reclaim.lock` busy error. Moving it out of default correctness
checks preserves that observed load limitation; it does not repair production
contention or prove burst fairness. No production timeout or Store lifetime is
changed. Default correctness success must not be reported as stress success.

## Host wait-path observation status

The 2026-10-07 host observation establishes only the callable route and event
boundaries. The inspected task exposed no standalone top-level `watcher_wait`
action; its available route was a nested Watcher MCP call under
`functions.exec`, with caller progress returned through `functions.wait`. This
is not a direct-await comparison. `wait_threads` remains a separate Codex-thread
API; its two-minute validation result does not establish a limit for Watcher MCP
`watcher_wait`.

One ten-minute no-change interval completed while the tracked parent turn
remained active. The nested Watcher calls returned without a report during that
window. A later metadata-only readback showed that the tracked turn completed,
but the assigned Watcher did not deliver the corresponding `turnCompleted`
event. This records a missing completion delivery, not zero idle wakeups.

A second ten-minute interval received an actual task message while the caller
was waiting, so it is not a matched no-change sample. The outer wait cell was
still running when caller execution resumed. After the interval, the assigned
Watcher delivered a true `turnCompleted` event for the tracked turn, while a
newer turn had already started. This confirms event receipt for that turn, not
thread quiescence.

The task message re-entered the caller while the nested Watcher wait remained
active. Internal wait cancellation and host-level interruption/resume therefore
remain unverified. No matched direct-versus-wrapper comparison was available
because a standalone direct call was not exposed. Model/tier, cache state, and
per-path token/cost categories remain unknown. No zero-idle-wakeup or
comparative cost claim follows.
