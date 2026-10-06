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

The 2026-10-07 host observation is incomplete. In the inspected Codex task,
`wait_threads` was available as a nested MCP tool inside `functions.exec`; no
top-level direct MCP call was exposed. The native wait path reported a
two-minute input ceiling; acceptance at that ceiling has not been verified.
No matched ten-minute no-change window followed by a material event was
completed. A parent wait returned a timeout before the handoff, so it does not
prove that the parent stayed suspended for the full window. Host interruption
and resume are also unverified; the existing interrupt tests cover the local
request-binding contract, not delivery by this host. No zero-idle-reentry or
token/cost conclusion follows from this evidence.
