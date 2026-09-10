# Child routing policy

Named packaged specialists are selected first and caller model overrides are
forbidden. Generic work defaults to `gpt-5.6-luna` at `max`; when that route is
unavailable, it fails closed to the root or named-specialist route. Simple work
uses the same Luna route when all simple predicates are complete.

For bounded native observation of assigned Codex Workers, the Orchestrator MUST
select the packaged `codexy-watcher` specialist and summon it through the host's
native subagent facility. It MUST NOT substitute a generic subagent or treat a
self-declared role name as specialist identity. The packaged Watcher declares
`gpt-5.6-luna` at `max`; caller overrides remain forbidden.

Thread delivery MUST bind `model` and `thinking` to the authenticated recipient,
not copy the sender settings. Parent-to-generic-child delivery MUST use
`gpt-5.6-luna` at `max`; child-to-root delivery MUST use `gpt-6-astra` at
`medium`. Both fields MUST be explicit. Unsupported or mismatched recipient
settings MUST fail closed instead of falling back to the sender route.

## Review routing

For a child-owned implementation lane, the owning child MUST delegate the
profile-selected internal reviewer after local proof when the selected profile
requires one. The parent MUST consume that evidence and MUST NOT invoke the
internal reviewer or tell the child not to invoke it. This is distinct from a
repository-required external `@codex review`, which the parent requests when
applicable; the child MUST NOT request that external review.

When private semantic evaluation is in scope, the owning child MUST complete
that independent evaluation and deliver its terminal summary, including any
unmeasured limitation, before delegating the selected reviewer. `PENDING`,
`RUNNING`, a bounded wait, or unavailable evaluator output is not a reviewer
verdict and MUST NOT be converted into `UNOBSERVABLE`. The reviewer assignment
MUST bind the evaluator result and the same frozen head. If a reviewer was
already started early, preserve its authentic event and natural terminal result,
record the ordering/evidence limitation, and do not interrupt, replace, or
duplicate it solely to repair the sequence.

Author self-review is forbidden, but delegating the independent packaged
reviewer is not self-review. The nonrecursive prohibition for a helper or
reviewer
(`MUST NOT spawn, delegate to, or create any additional agent, helper,
reviewer, task, or thread.`)
applies to that recipient and MUST NOT be copied into an owning-child assignment
as a ban on the child's required review.

Control-plane receipts and status handoffs MUST carry a stable `transition key`,
`event id`, or `state fingerprint` as applicable. Pre-delivery and post-result
receipts MUST use `transition key`. A completed delivery with the same
recipient, phase, and key MUST NOT be sent again; unchanged status MUST remain
silent.

Unknown, ambiguous, incomplete, and unsupported requests fail closed to the root
or named-specialist route. The executable contract is maintained by the packaged
runtime validator.
