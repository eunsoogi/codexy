# Token-Efficient Coordination

## Purpose

MUST keep Codexy coordination small without weakening evidence. MUST use this skill when a thread is recovering from compaction, receiving child terminal state, routing review feedback, or preparing a
handoff that might otherwise repeat large unchanged artifacts.

This skill is not a shortcut around `$proof-driven-completion`. It changes how evidence is summarized and refreshed, not which gates are required.

Live Sentinel observation MUST be read-only and event-driven. Generic child and ledger polling remains permitted. Both the child owner and the root orchestrator MUST NOT message, interrupt, replace,
duplicate, follow up with, or poll a live Sentinel. A bounded wait with no event is a non-terminal `PENDING` observation, and an independently observed live reviewer is `RUNNING`; neither observation
is a reviewer verdict or fallback-eligible. The owning lane MUST retain the same reviewer and wait for its natural terminal result. A live Sentinel MUST report its own terminal `PASS`, `BLOCK`, or
`UNOBSERVABLE` result naturally.

## Required Proof Gates

MUST NOT compress away these current facts for an active lane:

- issue and PR numbers,
- branch and worktree path,
- owner boundary and child thread id,
- current head SHA and base SHA,
- current check state,
- unresolved review thread ids and whether they are outdated,
- verification commands and results,
- merge readiness or explicit wait/stop condition.

Some lane gates may legitimately be absent, especially in issue-only, pre-PR, parent-owned, or pre-review loops. MUST record those slots explicitly as not-created or not-applicable states with a short
reason instead of inventing evidence or stalling the lane. MUST use those states only when the gate genuinely does not apply or has not been created yet. For gates that MUST exist for the current
lane, refresh existing gates directly instead of inferring them from older context.

## Event-driven delta

MUST use this flow after compaction and before handoff:

1. **Inventory once**: MUST keep one compact ledger line per active lane with `issue`, `PR`, `branch`, `head`, `owner`, and `state`.
2. **Accept qualifying events only**: root/orchestrator MUST NOT autonomously poll. The owner MUST use event-driven `wait_threads` with each target's latest cursor as the default for ordinary child
   completion or attention waits. The owner MUST reserve heartbeat scheduling for genuinely scheduled monitoring or when `wait_threads` is unavailable. After a host transition or
   `No handler registered` failure, the owner MUST treat the mismatch as host-transition exposure evidence, perform one fresh thread-tool discovery and one host-aware `wait_threads` retry before any
   fallback, MUST NOT use unbounded `read_thread`, and any bounded metadata fallback MUST consume the current parent-stage budget and record only returned size/token metadata. While a desktop-origin
   root turn has a callable `wait_threads` handler, the owner MUST keep ordinary child waits in a cursor-based `wait_threads` loop without finalizing that root turn between unchanged waits; mobile
   input that interrupts the active local wait MUST be consumed in that same local turn before the cursor-based wait continues. If a slingshot-host turn still returns `No handler registered` after the
   one fresh discovery and one host-aware retry, the owner MUST emit exactly one unavailable evidence receipt and require desktop-origin root re-entry; it MUST NOT repeat the wait call, schedule a
   heartbeat relay, use `read_thread`, or use `handoff_thread` for recovery. Children MUST send a compact delta only for terminal child state, Sentinel verdict, PR creation, new HEAD, GitHub
   check-state change, actionable review-feedback change, or review-thread resolution.
3. **Validate stable event identity**: every event MUST use a deterministic `<kind>|<lane>|<subject>` identity. The ledger MUST reject a repeated identity before it changes counters or next actions.
4. **Promote ids, not prose**: MUST keep exact ids and links. MUST NOT transfer a full conversation, full tool body, or full agent-tree listing. Direct reads and command output MUST remain bounded.
5. **Fail once**: a failed parent message MUST emit exactly one terminal unavailable report. It MUST include its event identity and MUST NOT retry the parent message.
6. **Carry one next action**: each lane MUST end with exactly one current action.
7. **Require runtime polling evidence**: polling/monitoring MUST be reserved for an observation bound to one complete runtime-issued monitor identity. A heartbeat route MUST bind the observation to
   its heartbeat automation id, target thread, bounded schedule, and last observed state fingerprint or event identity. The heartbeat route MUST NOT require a persistent exec/session identifier or
   same-process resume. A separate process-backed monitor MUST bind the observation to a persistent runtime monitor or wait session id, a scheduled next-observation time or deadline, the last observed
   state fingerprint or event identity, and same-process resume. Distinct model/assistant turn ids, tool-driven re-entry, goal continuation, or agent invocation without either complete runtime-issued
   identity are continuation turns, not polling; unchanged continuation turns MUST NOT reschedule themselves or emit another unchanged turn.
8. **Suppress unchanged continuation turns**: when an authorized child-local monitor observes no qualifying event and the stable event identity, head, checks, review state, and next action are
   unchanged, it MUST keep the monitor scheduled but MUST NOT emit a status message or start another model turn. The next scheduled read-only observation MAY run at its bounded interval. A new model
   turn may start only when that monitor observes a qualifying event, or when an explicit parent/user message arrives. This rule MUST NOT terminate or cancel the underlying wait/monitor session.

Before a child stops, archives, yields ownership, or calls `update_goal(complete)` or `update_goal(blocked)`, it MUST send exactly one terminal handoff delta to the source parent.
`update_goal(blocked)` additionally requires the typed unanswered user-decision gate; token pressure, repeated continuations, unchanged fingerprints, external producers, and coordination waits MUST
NOT authorize a blocked goal. That delta MUST include the stable event identity, issue/PR, child task id, branch/worktree, exact HEAD and dirty/index state, last completed proof, current external
gate, preserved artifacts or reservation, and one parent-owned next action. The child MUST confirm task-surface delivery before the stop/archive or goal transition. A failed delivery MUST emit one
unavailable receipt and MUST NOT retry or transition.

## Event Delta Shape

MUST use this compact shape for each lane:

```text
#<issue> / PR #<pr> / <branch>
event id: <kind>|<lane>|<subject>
event kind: terminal-child | sentinel | pr-created | new-head | check-state | review-feedback | review-clean | unavailable
owner: child thread <id> | worktree <path>
head: <sha> | base: <sha>
delta: <one changed fact>
required gates: checks=<state>; threads=<state>; child=<state>
active obligations: <only current unresolved work>
stale/demoted: <old heads, resolved threads, superseded comments>
next action: <one action>
```

When no qualifying event arrived, MUST NOT wake the implementation lane. The orchestrator MAY retain its compact ledger without re-reading old details.

## Runtime Heartbeats

For an eligible external gate that outlives the current turn, parent orchestrators and child owners MUST follow `$orchestration`'s runtime-heartbeat contract. The compact lane ledger MUST retain the
heartbeat automation id, target thread, bounded schedule, state fingerprint, material-event set, and delete/disable state. Heartbeat prompts MUST suppress unchanged observations and MUST wake the
owner only for a material gate change or an explicit user/parent message. A stable event identity MUST deduplicate repeated wakeups before the owner changes its plan. The awakened owner MUST consume a
material event in the same turn and MUST delete or disable its heartbeat when no further observation is required. A successfully registered heartbeat is runtime-owned waiting. The owner MUST retain
its active goal and plan only while an immediately executable in-scope obligation remains, record `goal state=active` and `goal transition=none`, and return control without completing or blocking the
goal. When only an external event or explicit parent wake remains, the owner MUST use the idle-wait handoff, complete the finite goal, and leave the task idle without claiming the issue complete. A
qualifying event MUST create a fresh short-lived execution goal and current plan before any edit, proof, review response, publication, or merge work. A live packaged Sentinel remains outside heartbeat
observation and retains its no-poll/no-message boundary.

For repeat handoffs, copy [the delta-poll template](../templates/delta-poll.md) and fill only the current slots. MUST keep the template output in the thread or handoff; MUST NOT attach old logs or
unchanged review bodies unless a current gate points to them.

## Metadata-Only Session Audit

In a source checkout, MUST use `scripts/session-audit --input <metadata-jsonl>` for bounded aggregate evidence. An installed skill MUST NOT claim this repository-local command is packaged; it MUST
direct users to the source checkout or an explicitly packaged runtime before requesting audit execution. The audit MUST report session size, latest cumulative tokens, recent per-turn average, call
counts by tool, and output bytes by tool. It MUST read only exact top-level metadata keys, reject invalid ids or tool keys, deduplicate the stable event identity, and MUST NOT emit prompts, tool
arguments, tool bodies, or nested metadata. For string output, `output_bytes` is decoded UTF-8 byte length; for arrays, objects, and scalars it is compact JSON UTF-8 serialization length, not the
source JSONL/wire length or Unicode character count. The audit MUST accept one session only, bind a tool name to its first valid `(session, call-kind, call-id)`, count the first matching output once,
reject conflicting bindings, and ignore orphan outputs.

MUST capture before/after aggregate output for one real lane using a comparable window and owner boundary. MUST use [the session-audit proof receipt](../templates/session-audit-proof-receipt.json) as
the metadata-only receipt: it MUST include review feedback, child age, retries per PR, stable event ids, goal/plan receipts, helper ownership, sanitized audit input digest, and command exits. The
comparison MUST report observations only; it MUST NOT claim a causal driver without a controlled comparison. Historical text, negated feedback, and stale-head events MUST NOT count as current review
activity.

Before attributing runtime behavior, MUST establish installed content equivalence, not just a matching version. MUST read the candidate manifest version, MUST run `codex plugin add codexy@codexy` to
update the configured marketplace install, then compare the manifest and every changed packaged skill/template by SHA-256 or `cmp`. MUST record `codex plugin list` output, the installed cache root,
changed-file digests, and the metadata-only before/after audit. If an explicitly packaged runtime command is absent or differs, MUST record it as an install failure and MUST NOT attribute the
candidate behavior to the installed plugin.

## Compaction Budget

After compaction, rebuild only the working set:

- active lanes and their latest known SHAs,
- unresolved review thread ids,
- child ownership and stop condition,
- commands already run only when their result still proves a current gate,
- known tool exposure mismatches that affect the next action.

MUST NOT reload old full review bodies, full command output, resolved feedback, or closed lanes unless a current gate references them.

## Handoff Discipline

For a child handoff or parent status, include:

- `remember`: durable facts needed for the next gate,
- `refresh`: facts that MUST be re-polled before action,
- `forget`: resolved, outdated, superseded, or irrelevant details,
- `next`: one action with the owner.

MUST use `forget` for stale context. It means the detail MUST NOT drive active work unless a fresh poll makes it current again.

## Stop Conditions

MUST stop and refresh rather than summarizing when a qualifying event reports:

- the head SHA changed,
- a check moved from pending to pass/fail;
- a review thread changed resolved or outdated state;
- child ownership is unclear; or
- the next action would merge, resolve a review thread, or claim readiness.

These actions MUST require current authoritative evidence, not a cached summary.
