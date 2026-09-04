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

After full and delta are both consumed, exactly one third
`required_current_head` review may be admitted when the current head moved for
mandatory base integration or an in-scope contract/root repair. It MUST use the
same selected reviewer and carry a typed `post_cap_re_review` reason plus the
prior delta head. The marker MUST carry a qualifying-change object whose
`from_head` is the delta head, whose `to_head` is the current head, and whose
`evidence_commit` is an ancestor between them. Mandatory base integration MUST
change `baseRefOid` and prove base and integration ancestry. Contract/root
repair MUST preserve `baseRefOid`, require a prior `BLOCK` delta with non-empty
findings, and bind `finding_ids` exactly to those findings. Optional churn,
duplicate or unchanged heads, missing/reordered/truncated history, and a fourth
terminal verdict MUST be rejected. A third `BLOCK` permits only the bounded
issue-contract/root repair and refreshed exact-head proof; a third
`UNOBSERVABLE` requires maintainer disposition and current proof.

The third verdict does not authorize completion by itself. Exact-head `PASS`, no
unresolved findings, tests, validators, CI, review-thread, ownership, safety,
LOC, and merge gates remain required. Both third-result paths waive only review
four, which MUST NOT occur. Reviewer MUST NOT be messaged, interrupted,
replaced, duplicated, or polled.
