Reviewer MUST use the current diff/head. `PASS`, `BLOCK`, and `UNOBSERVABLE` are
terminal; `PENDING` and `RUNNING` are non-terminal observations of the same
active reviewer and do not consume a verdict.

The direct `codexy.review-control-state.v1` state MUST carry the issue identity,
terminal review count, a three-verdict limit, and the ordered terminal history.
The owner MUST preserve that history across goals, lanes, compaction,
reauthorization, and route resets. Full remains one review and delta remains at
most one recheck; the counters and history MUST not be reset or silently
discarded.

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
MAY use one complete `codexy.review-control-pre-pr-history.v1` envelope to import
the original final-message identity, turn/order references, reviewer facts, and
terminal history into a genesis PR state. The source adapter MUST use an actual
Codex readback, or the exact original host record when a completed `read_thread`
turn omits its items; an empty turn MUST NOT be treated as evidence. The runtime
validates structure, issue binding, reviewer policy, and Git ancestry but does not
authenticate credentials or derive a verdict from a caller flag or signature.
Import MUST preserve the current PR number, URL, base, and head, MUST reject an
existing history, and MUST leave a compact immutable `pre_pr_import` marker. An
older imported PASS is bookkeeping only: readiness still requires a PASS at the
actual current head. Later ordinary transitions MUST preserve the marker and
reject changed, removed, reordered, duplicated, or incomplete provenance.

After full and delta are both consumed, exactly one third
`required_current_head` review may be admitted when the current head moved for
mandatory base integration, an in-scope contract/root repair, or an
authenticated external finding discovered on the clean delta-PASS head. It
MUST use the current policy reviewer (with any previously authenticated
migration marker preserved) and carry a typed `post_cap_re_review` reason plus
the prior delta head. The marker MUST carry a qualifying-change object whose
`from_head` is the delta head, whose `to_head` is the current head, and whose
`evidence_commit` is an ancestor between them. Mandatory base integration MUST
change `baseRefOid` and prove base and integration ancestry. Contract/root
repair MUST preserve `baseRefOid`, require a prior `BLOCK` delta with non-empty
findings, bind `finding_ids` exactly to those findings, and show the evidence
diff changes every finding's recorded path. Authenticated external finding
repair MUST preserve `baseRefOid`, require a clean prior `PASS` delta with no
unresolved findings, and bind a source envelope captured by authenticated
GitHub GraphQL. That envelope MUST bind the source repository, owning issue,
PR, review-thread/comment identity, author, observed commit equal to the delta
head, unique finding IDs, and repository-relative paths; the repair diff MUST
touch every recorded path. The source PR's owning issue is provenance and does
not replace the target control issue. Independent evaluator output remains
unavailable unless a trusted adapter exposes a concrete safe source with the
same path/head binding and no private inputs, answers, or artifact paths; a
public `FAIL` word alone is not evidence. Optional churn, duplicate or unchanged
heads, missing/reordered/truncated history, and a fourth terminal verdict MUST
be rejected. A third `BLOCK` permits only the bounded issue-contract/root repair
and refreshed exact-head proof; a third `UNOBSERVABLE` requires maintainer
disposition and current proof.

The third verdict does not authorize completion by itself. Exact-head `PASS`, no
unresolved findings, tests, validators, CI, review-thread, ownership, safety,
LOC, and merge gates remain required. Both third-result paths waive only review
four, which MUST NOT occur. Reviewer MUST NOT be messaged, interrupted,
replaced, duplicated, or polled.
