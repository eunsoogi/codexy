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

The 2026-10-07 host measurement remains incomplete. `wait_threads` is a
separate Codex-thread wait surface; its two-minute input validation result
applies only to `wait_threads` and does not establish a limit for Watcher MCP
`watcher_wait`. The inspected task exposed `watcher_wait` only as a nested tool
under `functions.exec`; no standalone direct `watcher_wait` call was available.
The Watcher MCP contract separately allows waits up to its documented
sixty-minute maximum. The direct `wait_threads` API is not a comparable
substitute because it waits for Codex threads rather than Watcher reports.

A separate parent observation used short segmented Watcher waits and received
one event. It came from a different session and call pattern, so it is not this
lane's matched no-change window or an equivalent direct-call comparison. The
lane's prior Watcher session was durably canceled during handoff; it was not
reused, and no new session was opened.

No matched ten-minute no-change window followed by a material event was
completed by this lane. A ten-minute run split across short `functions.exec`
waits could measure only that segmented pattern, with a parent return per
segment; it would not match a single long direct wait. No such segmented run
was completed here. Host interruption and resume remain unverified; the
existing interrupt tests cover local request-binding, not delivery by this
host. No zero-idle-reentry or token/cost conclusion follows from this evidence.
