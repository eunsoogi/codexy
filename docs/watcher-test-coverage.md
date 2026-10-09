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

## Single wait response envelope

The normal `watcher_wait` JSON envelope contains only `status`, `sessionId`,
`nextCursor`, `events`, and `health`. The health object always has exactly
`status`, `watcherState`, `lastObservationAtMs`, and `lastError`, preserving
null observation times, unknown states, and errors. The wait `status` retains
its event, timeout, cancelled, and expired meanings; health `status` remains
active, cancelled, or expired. A returned `cancelled` result additionally has
`cancellationReason`: `request_cancelled` for request-binding interruption or
`session_cancelled` for durable `watcher_cancel`. Same-connection MCP
cancellation can suppress the JSON result and does not gain a synthetic success
response.

Wait health no longer repeats `actor`, `sessionId`, `assignmentId`, `parent`,
`watcher`, `targets`, `generation`, `queueDepth`, `lastMaterialEventAtMs`,
`waiting`, `transport`, `transportConnected`, `nativeStatus`, or `expiresAtMs`
inside health. `watcher_health` retains those detailed diagnostics.
In-repository callers that need them MUST make an explicit `watcher_health`
call; they MUST NOT assume every wait includes them or add an automatic health
request after each wait. The expired-session test now checks `queueDepth`
through the explicit health result.

### Event projection and migration

Every `watcher_wait.events[]` entry contains exactly `eventId`, `sequence`,
`kind`, `target`, `summary`, `observedAtMs`, and `evidence`. The values and
event order are preserved in full. The response no longer includes
`fingerprint`; that value remains in durable event records for retry and
explicit-ID conflict validation. Existing stored records replay without a schema
migration or an identity change.

Consumers that read `fingerprint` from wait events MUST migrate to the
seven-field response contract and use the unchanged `eventId` for public event
identity. This note describes the intended single-response change and does not
claim that every external consumer has been audited for compatibility.

The deterministic event test uses identical fixed inputs with one report and an
eight-report page, including Korean and multibyte values. It measures payload
JSON and the serialized MCP `result.content` bytes separately:

| Reports | Baseline payload | Candidate payload | Payload reduction | Baseline MCP content | Candidate MCP content | MCP content reduction |
| ------: | ---------------: | ----------------: | ----------------: | -------------------: | --------------------: | --------------------: |
|       1 |              514 |               433 |             15.8% |                  601 |                   516 |                 14.1% |
|       8 |             2852 |              2204 |             22.7% |                 3163 |                  2483 |                 21.5% |

The baseline bytes were captured on the integrated #1306 base
`6a0edb09affcb93db4e01f5c35270164d6d0a6c2` before the response projection. That
characterization run intentionally failed the new seven-field assertion because
the predecessor still returned `fingerprint`. The candidate measurement uses the
same fixture and passed:

```sh
cargo test --locked --manifest-path packages/codexy-runtime/Cargo.toml --test suite_system system::mcp_stdio::watcher_deterministic::watcher_projection::event_wait_projection_preserves_full_values_and_reduces_payloads -- --exact --nocapture
```

These byte counts describe serialized response size only; they do not measure
tokenizer behavior, model use, or billing.

The deterministic stdio test uses fixed session IDs, targets, expiry, and health
values for one-target and eight-target empty timeouts. It counts UTF-8 bytes in
the returned payload JSON and in the serialized MCP `result.content` array
without normalizing removed fields:

| Targets | Baseline payload | Baseline MCP content | Candidate payload | Candidate MCP content | Payload reduction |
| ------: | ---------------: | -------------------: | ----------------: | --------------------: | ----------------: |
|       1 |              514 |                  619 |               171 |                   226 |             66.7% |
|       8 |              682 |                  815 |               171 |                   226 |             74.9% |

The baseline was measured at `d11327aaae041830d7d6b4efa08ecc80b0ab31ca` before
the production response edit with:

```sh
cargo test --locked --manifest-path packages/codexy-runtime/Cargo.toml --test suite_system system::mcp_stdio::watcher_deterministic::empty_wait_response_sizes_are_measured_for_one_and_eight_targets -- --exact --nocapture
```

That run passed and reported payload/content byte counts of `514/619` for one
target and `682/815` for eight targets. The candidate uses the same real stdio
fixture; its current command is:

```sh
cargo test --locked --manifest-path packages/codexy-runtime/Cargo.toml --test suite_system system::mcp_stdio::watcher_deterministic::watcher_response::empty_wait_response_sizes_are_measured_for_one_and_eight_targets -- --exact --nocapture
```

The baseline raw stdout was not saved as a separate file; the command, result,
revision, and counts above are transcribed from the task's recorded baseline
run. These byte counts describe the serialized response only. They do not
measure tokenizer, model, or billing savings.

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

## Historical 2026-10-07 host wait-path observations

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

## #1312 synchronous host-wait evidence (2026-10-10)

### Route and material events

The issue-reported failure was an outer `functions.exec` default yield that
returned `Script running with cell ID` while nested `watcher_wait` remained
pending. The parent then retrieved the cell with `functions.wait`, resumed model
reasoning, emitted commentary and performed unrelated cleanup observation. This
is the issue's reported baseline, not a replay performed for this change.

On the verified host, no standalone top-level `watcher_wait` action was exposed.
Each measured call used one `functions.exec` with first-line
`// @exec: {"yield_time_ms":300000}` and one nested
`watcher_wait(timeoutMs=295000)`. The MCP wait, the outer `functions.exec`
yield, host scheduling and model-visible parent turn are separate boundaries.
The Responses API's `async: true` metadata is not a Codex host control; see the
[official async tool-calling guide](https://developers.openai.com/api/docs/guides/async-tool-calling).

Parent-supplied sanitized receipts cover one production session with the same
parent and assigned native Watcher identities. The session ID value was not
included in the sanitized receipt. Each call returned one genuine event
directly, with no intermediate cell response or duplicate/replay waiter:

- Cursor `0` → `1`; `gate_ready` for the #1312 Worker; 32.7 s;
  `observedAtMs=1791563583274`; `eventId`:
  `evt-cc5933d64b5b5ca0e4e5520ebb9e2b80b6e6ba60ac491de28ec09f2b2331314e`.
- Cursor `1` → `2`; `unavailable` for the #1306 cleanup Worker; 140.7 s;
  `observedAtMs=1791563784852`; `eventId`:
  `evt-79a5b0e6e9c172011f9d93654871a7771a461ad57df6aaa0dd8de7a900edbe35`.
- Cursor `2` → `3`; `gate_ready` for the #1306 cleanup Worker; 228.3 s;
  `observedAtMs=1791564069515`; `eventId`:
  `evt-7f64fcd33049a77136b617120443fc4aca6115bf98a6ff81c79996b31d36e77d`.

The #1306 cleanup observations were delivered through the assigned Watcher; the
parent did not use a direct `wait_threads` bypass in these calls.

Two subsequent parent waits on cursor `3` returned `status=timeout`, empty
events and unchanged cursor after 295.3 s and 296.1 s. The parent observed no
intermediate wrapper response or host-visible model, tool, or commentary item
before either final result. Hidden inference is not observable. These were
production observations on installed Codexy 1.13.2 before these candidate
instructions were installed; they verify the route on this host, not behavior on
another host or hidden model state.

### Interruption and runtime invariants

On the same route at cursor `3`, the parent reports that the human user
confirmed interruption and the wait returned `status=cancelled`, empty events
and unchanged cursor `3`. A fresh `watcher_health` read remained active, with
`waiting=false`; the durable session was not cancelled. A child terminal message
arrived near the same time, and the outer call ended at 27.9 s. The available
timeline cannot isolate the user's input as the sole cancellation cause. This
records a real host cancellation result and preserved session, but does not
claim exclusive causal instrumentation.

Runtime tests continue to cover the distinct outcomes. The Rust runtime and
system suites in [PR #1311](https://github.com/eunsoogi/codexy/pull/1311) passed
on head `0633359204960673de7cc1cfe347263a6e02c41f`, merged as
`6a0edb09affcb93db4e01f5c35270164d6d0a6c2`. Since that head,
`request_binding.rs`, `wait.rs`, `watcher_protocol.rs`, `watcher_long_wait.rs`,
and the `watcher_interrupt` tests are unchanged. PR #1313 changes the public
event projection in `response.rs`; its cancellation-reason mapping is unchanged.
The current protocol test verifies request cancellation leaves the durable
session active, and the long-wait tests distinguish `session_cancelled` from
`expired` while preserving cursor and queued data. These tests support runtime
lifecycle behavior; they do not substitute for the host observations above or
prove that a real user interruption was the sole cause in the raced host
observation.

## #1308 installed-agent instruction observation (2026-10-09 UTC)

A bounded observation used installed Codexy 1.13.2 `codexy-watcher`, configured
`gpt-6-luna/max` (effective settings unavailable), with the three candidate
guidance files at `b2f66bfb328ed0f883c461dbd23b86b32115343b`. This is
installed-agent use with supplied candidate guidance, not installation of a
package built from that candidate.

- Two `wait_threads` calls on one assigned target/turn advanced cursors
  `84→86→88`. The first complete status/message needed no parent action and
  caused no `read_thread`, `watcher_report`, or `watcher_health`.
- The second result had a new failed-command marker but no result. One recent
  `read_thread` without a cursor and with `includeOutputs=true` retrieved it;
  the docs/lint lookup exited 2. This was an actual command failure, not a
  production fault or an accepted blocker.
- Totals were `wait_threads=2`, `read_thread=1`, `watcher_health=0`. Two later
  `watcher_report` calls covered a required metadata-only update and requested
  health delivery. `watcherState=running` returned `health_updated` without a
  queued wake; a parent timeout at cursor `3` showed `queueDepth=3`, running
  state, and an updated observation timestamp. The requested `health` event
  arrived once at cursor `3→4`,
  `eventId=evt-16aa302fcefc7d5ad66f566b1a9de0d3d329f74f2551fe1822933ed94e71513d`,
  `observedAtMs=1791569576338` (`2026-10-09T18:12:56.338Z`).

This sample did not cover contradictory or unbound results, retained terminal
text omitted by an up-to-date cursor, or final reviewed-PR terminal delivery.
The sample does not prove candidate-package installation, effective model
settings, hidden inference, or token/billing cost.
