Reviewer MUST use the current diff/head. `PASS`, `BLOCK`, and `UNOBSERVABLE` are
terminal; `PENDING` and `RUNNING` are non-terminal observations of the same
active reviewer and do not consume a verdict.

The direct `codexy.review-control-state.v1` state MUST carry the issue identity,
terminal review count, a three-verdict limit, and the ordered terminal history.
The owner MUST preserve that history across goals, lanes, compaction,
reauthorization, and route resets. On ordinary admitted routes, full remains one
review and delta remains at most one recheck; the counters and history MUST not
be reset or silently discarded.

## Review ownership

For a child-owned lane, the branch-owning child owns profile-selected internal
review delegation, the wait for its natural terminal result, review-feedback
repair, and native review-history preservation. The parent consumes that
evidence, requests a separate repository-required external `@codex review` when
applicable, and makes integration and readiness decisions. The parent MUST NOT
invoke the child lane's internal reviewer or tell the child not to invoke it.

The author MUST NOT review the author's own diff or present self-inspection as
the profile-selected gate. Delegating the independent packaged reviewer is
required when the selected profile requires one and is not self-review.

If a parent has already summoned that internal reviewer for a child-owned lane,
the owning child MUST preserve the authentic host event and existing direct
review state. Any admitted terminal result MUST remain counted exactly once in
`terminal_review_history`, with the actual sender, receiver, model, effort,
reviewed head, result, and findings retained. The child MUST NOT reset history
or create a duplicate fresh full review solely to repair the routing mistake.
When a durable representation is missing, use the existing native-history
capture/recovery path; preserve `not_attested` or `not_admitted` limitations and
never fabricate a child sender or reviewer verdict.

When private semantic evaluation is in scope, the owning child MUST wait for its
independent evaluator to reach a terminal result and deliver its compact
summary, including any unmeasured limitation, before delegating the selected
reviewer. `PENDING`, `RUNNING`, a bounded wait, or unavailable evaluator output
is not a reviewer verdict and MUST NOT be converted into `UNOBSERVABLE` merely
because the reviewer lacks the result. The reviewer assignment MUST bind the
same frozen head and the evaluator's available result. If a reviewer was already
started before that evidence arrived, preserve its authentic event and natural
terminal result, record the sequencing/evidence limitation, and do not
interrupt, replace, or duplicate it solely to repair the ordering.

Missing historical evidence MUST be treated as a limitation on the review or
readiness result, not automatically as a limitation on all authorized work. The
agent MUST preserve the actual unknown, `UNOBSERVABLE`, `not_attested`, or
`not_admitted` state and MUST NOT turn it into a new approval request. The agent
MUST continue only the next action already covered by the current request or
authorization and MUST withhold claims the evidence cannot support.

Every reviewer-backed state transition MUST use authenticated current and
previous PR snapshots from the canonical GitHub readback producer. The snapshots
MUST bind the same repository, PR number, URL, base branch, and capture
provenance, with direct `baseRefOid` and `headRefOid` values. The previous
snapshot's `reviewControl` is the only predecessor authority; a separate
`previous_control_state` input MUST be rejected. The first full event MUST
append to a clean zero-count genesis; each later state MUST preserve the exact
prior history prefix and increase the terminal count by one. A fresh one-event
input MUST NOT reset prior terminal history, and the validator MUST preserve the
current snapshot's authenticated head and base values.

When a selected reviewer completed before PR creation, the trusted orchestrator
MAY use one complete `codexy.review-control-pre-pr-history.v1` envelope to
import the original final-message identity, turn/order references, reviewer
facts, and terminal history into a genesis PR state. The source adapter MUST use
an actual Codex readback, or the exact original host record when a completed
`read_thread` turn omits its items; an empty turn MUST NOT be treated as
evidence. The runtime validates structure, issue binding, reviewer policy, and
Git ancestry but does not authenticate credentials or derive a verdict from a
caller flag or signature. Import MUST preserve the current PR number, URL, base,
and head, MUST reject an existing history, and MUST leave a compact immutable
`pre_pr_import` marker. An older imported PASS is bookkeeping only: readiness
still requires a PASS at the actual current head. Later ordinary transitions
MUST preserve the marker and reject changed, removed, reordered, duplicated, or
incomplete provenance.

A separate pre-PR preservation route MAY use the marker with
`pre_pr_import.mode=preserved_history` and
`pre_pr_import.admission=not_admitted`. It MUST preserve the complete ordered
terminal history as actually observed, including every valid event kind, result,
finding, and history longer than the ordinary issue-wide three-event quota. It
MUST retain actual event IDs, threads, turns, ordinals, heads, results,
findings, source provenance, and the distinction between policy and invocation
attestation. This includes sequences such as `BLOCK→BLOCK→PASS`; an unknown or
absent kind MUST remain unknown or absent and MUST NOT be relabeled as
`required_current_head`. The route MUST NOT hide over-quota events, rewrite the
last `PASS`, delete an earlier `BLOCK`, auto-admit the last `PASS`, accept a
synthetic event, or change the ordinary profile quota. Preserved history is
bookkeeping only: it MUST NOT authorize readiness, quota consumption, merge,
completion, or another review. Only the existing authenticated final-disposition
or consumer logic may later admit or dispose of it with current-head proof. The
marker and source remain shape evidence, not credential authentication.

When a selected reviewer completed after PR creation and the supported host
records remain available, the owner MAY use the native recovery CLI with one
complete owner/reviewer capture and a fresh authenticated current PR snapshot:
`codexy-review-control --recover-native-review-history
--current-pr-state-file <current> --input <native-history> --output <recovered>`.
The capture MUST preserve the completed owner spawn, its single reviewer
receiver, every complete reviewer page, source-local order, final message
identity, reviewed head, terminal result, findings, and unchanged raw UTF-8
source. A read_thread projection MAY be supplemented only by a separate
validated native host record when `function_call`/`call_id`, the matching
`function_call_output`, owner session/thread, and `SubAgentActivity`
receiver/path/selected role bind exactly. Encrypted prompt, model, or effort
fields remain unknown and MUST NOT be caller-supplied. The input MUST NOT claim
authentication or supply a current PR snapshot; raw caller JSON is shape input,
not credential proof. The CLI binds the supplied snapshot and records the result
as `proved_post_pr`, `not_attested`, and `not_admitted`. Recovery MUST accept
only one full event followed by an optional delta, MUST reject an existing
history, and MUST retain the immutable top-level `nativeHistoryRecovery`
receipt. The recovered control MUST remain non-admissible while its
`native_history_recovery` blocker is present. Only a subsequent ordinary
current-head transition may append its real verdict and remove that blocker; the
`native_history_provenance` marker and full receipt MUST be carried forward and
revalidated against the preserved source. A recovery receipt alone MUST NOT
authorize readiness, completion, merge, or another review.

After full and delta are both consumed, exactly one third
`required_current_head` review may be admitted when the current head moved for
mandatory base integration, an in-scope contract/root repair, an authenticated
external finding discovered on the clean delta-PASS head, or an authenticated
mixed-finding disposition from a blocked delta. It MUST use the current policy
reviewer (with any previously authenticated migration marker preserved) and
carry a typed `post_cap_re_review` reason plus the prior delta head taken from
`terminal_review_history[1].reviewed_head`, never a substitute snapshot head.
The marker MUST carry a qualifying-change object whose `from_head` is the delta
head, whose `to_head` is the current head, and whose `evidence_commit` is an
ancestor between them. Mandatory base integration MUST change `baseRefOid` and
prove base and integration ancestry. Contract/root repair MUST preserve
`baseRefOid`, require a prior `BLOCK` delta with non-empty findings, bind
`finding_ids` exactly to those findings, and show the evidence diff changes
every finding's recorded path. Authenticated external finding repair MUST
preserve `baseRefOid`, require a clean prior `PASS` delta with no unresolved
findings, and be produced from a locator-only
`authenticated_external_finding_locator` request. The producer MUST perform a
fixed-argument, host-authorized GitHub GraphQL read, persist its raw response
and deterministic projection in the source envelope, and reject caller-supplied
source or capture values. `capture.raw` equality and re-projection are offline
shape/integrity checks only, not authentication. The producer, `build-pr-state`,
and completion handoff MUST use the live source read for external-finding
authority; offline validators only validate an envelope already admitted by that
boundary. The envelope MUST bind the source repository, owning issue, PR,
review-thread/comment identity, author, observed commit equal to the delta head,
unique finding IDs, and repository-relative paths to the live projection; the
repair diff MUST touch every recorded path. The source PR's owning issue is
provenance and does not replace the target control issue. Independent evaluator
output remains unavailable unless a trusted adapter exposes a concrete safe
source with the same path/head binding and no private inputs, answers, or
artifact paths; a public `FAIL` word alone is not evidence. Optional churn,
duplicate or unchanged heads, missing/reordered/truncated history, and a fourth
terminal verdict MUST be rejected. A third `BLOCK` permits only the bounded
issue-contract/root repair and refreshed exact-head proof; a third
`UNOBSERVABLE` requires maintainer disposition and current proof.

The mixed-finding disposition MUST preserve the unchanged base, bind every prior
delta finding exactly once, use the locator-only
`authenticated_finding_disposition_locator`, and refresh its authenticated CI
rollup and maintainer decision at production, build, and handoff. Classification
MUST follow the retained finding's semantic kind, not a path prefix: CI
observations require a non-empty exact-head all-success CheckRun rollup; a
source defect under a workflow path still requires code repair; the policy
finding requires an immutable, unminimized OWNER/MEMBER comment bound to the
exact repository, owning issue, PR, base, head, finding ID/path, and accepted
model tuple. Other findings require an evidence diff touching their exact
repository-relative paths, and at least one such code repair is mandatory.
Caller-supplied source, capture, classification, and IDs are rejected; producer,
build, and handoff MUST refresh and rederive the live classification before
comparison. The disposition never waives code, CI, review, merge, or quota
requirements. The third verdict does not authorize completion by itself.

After an authentic third `BLOCK`, the sibling `final_disposition` object MAY
record one bounded parent/maintainer disposition without creating a fourth
review event or rewriting the third result. It MUST preserve the immutable
three-event history, `terminal_result = BLOCK`, and `terminal_review_count = 3`.
For a repaired source head, `source_repair` MUST bind the third head to an
ancestor evidence commit and then to the exact current head; its evidence and
final-tree diffs MUST be non-empty, limited to the selected finding paths, and
retain each change at the current head. For a same-head correction,
`evidence_refresh` MUST be used instead, with no artificial source edit.

The parent authority MUST be produced from an authenticated
`authenticated_final_disposition_locator`, never caller-supplied. Producer,
`build-pr-state`, and handoff MUST reread the live source and bind its
OWNER/MEMBER immutable comment and exact-head all-success CI to the repository,
owning issue, PR, base, current head, third review event, and finding set.
Complete paginated review-thread evidence with zero unresolved threads remains
required. This path does not permit synthetic `PASS` or `UNOBSERVABLE`, a fourth
profile review, private evaluator inputs, or any waiver of ordinary tests,
ownership, safety, LOC, connector-review, CI, or merge gates. Outside
`final_disposition`, exact-head reviewer `PASS` and no unresolved reviewer
findings, together with tests, validators, CI, review-thread, ownership, safety,
LOC, and merge gates, remain required. With `final_disposition`, the reviewer
projection remains the authentic third `BLOCK`; its live authority MUST still
prove exact-head all-success CI and complete resolved review-thread evidence,
while the same ordinary gates remain active. Both final-disposition forms waive
only review four, which MUST NOT occur. Reviewer MUST NOT be messaged,
interrupted, replaced, duplicated, or polled.
