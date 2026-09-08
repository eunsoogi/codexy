# Review profiles

The closed review profile set is:

- `light`: no reviewer and no full or delta recheck quota.
- `standard`: one `codexy-inspector` review and one full plus one delta recheck;
  one typed current-head re-review may consume terminal verdict three after
  those two reviews.
- `strict`: one `codexy-sentinel` review and one full plus one delta recheck;
  one typed current-head re-review may consume terminal verdict three after
  those two reviews, while blocking findings remain bounded by the strict
  contract.

The post-cap re-review is not another full or delta quota. It is admitted only
from the direct ordered terminal history, for mandatory base integration, an
in-scope contract/root repair, an authenticated external finding discovered on
the clean delta-PASS head, or an authenticated mixed-finding disposition from a
blocked delta; the issue-wide terminal limit remains three. A migration marker
MUST preserve actual reviewer tuples and explicitly select the normal
legacy-prefix/current-suffix direction or the boundary-1 current-prefix/legacy-
delta exception; it MUST NOT rewrite history.

Reviewer-backed transitions use authenticated current and previous PR snapshots
from the canonical GitHub readback producer. Snapshots bind the same repository,
PR number, URL, base branch, capture provenance, `baseRefOid`, and `headRefOid`;
the authenticated `capture.owningIssue` object also binds the owning issue's
repository, number, canonical URL, and explicit `owner-assignment`,
`closing-issue-reference`, or `linked-issue-reference` association. The owning
issue object comes from the authenticated issue read and is distinct from the PR
number; `reviewControl.issue_number` binds that owning issue. The previous
snapshot's `reviewControl` is the only predecessor authority.
`previous_control_state` is rejected. Base integration must change and prove
base ancestry. Contract/root repair must retain the base, follow a prior `BLOCK`
delta with findings, bind `qualifying_change.finding_ids` exactly to those
findings, and show the evidence diff changes every finding's recorded path. The
current snapshot's head and base identity are preserved.

The supported connector capture MUST follow the
[authenticated GitHub connector capture](connector-capture.md) contract,
including its source, derivation, contradiction, and live-proof boundaries.

The external-finding reason MUST be produced from a locator-only
`authenticated_external_finding_locator` request. The producer MUST perform a
fixed-argument, host-authorized GitHub GraphQL read for that locator, reject
command failures, GraphQL errors, incomplete connections, and identity
mismatches, then construct the `codexy.review-control-external-finding.v1`
envelope with the raw response and its deterministic projection. Caller-supplied
`authenticated_external_finding` or `authenticated_external_finding_capture`
values MUST be rejected. `capture.raw` equality and re-projection are offline
shape/integrity checks only and MUST NOT be treated as authentication. The
producer, `build-pr-state`, and completion handoff MUST use the live source read
for external-finding authority; offline validators only validate an envelope
already admitted by that source-owned boundary. The envelope's repository,
owning issue, source PR, immutable review-thread and comment identities with the
canonical discussion URL, author, observed commit, unique finding IDs, and
repository-relative affected paths MUST equal the live projection. The
transition requires `observedCommit` to equal the prior delta head and the
repair diff to touch every recorded path. A source with different repository,
issue, PR, head, finding set, or paths is rejected. The source PR's owning issue
is provenance and does not replace the target `reviewControl.issue_number`.

The mixed-finding disposition reason MUST preserve the base OID and cover every
finding from the blocked delta exactly once. It MUST be produced only from a
locator-only `authenticated_finding_disposition_locator` request. Its exact
authenticated CI and maintainer-source contract is defined in
[authenticated finding-disposition CI](finding-disposition-ci.md). `known_empty`
required checks are an authenticated state, not a future-job inventory;
unavailable or incomplete sources remain unproved. Finding classification MUST
follow the retained semantic kind, not a workflow path: CI observations resolve
only through CI, a source defect still requires actual evidence-diff path
coverage, and the exact policy finding only through the maintainer decision. All
remaining findings require actual evidence-diff path coverage with at least one
code repair. The producer, `build-pr-state`, and completion handoff MUST reread
both sources; callers MUST NOT provide source, capture, classification, or
finding IDs, and this reason MUST NOT waive code, CI, review, merge, or quota
requirements.

After a third terminal `BLOCK`, the sibling `final_disposition` path MAY record
the parent's bounded disposition while retaining all three original review
events and `terminal_review_count = 3`. A repaired-head disposition MUST bind
the third head through an ancestor evidence commit to the exact current head,
with non-empty, finding-path-only diffs retained in the final tree. A same-head
disposition MUST use evidence refresh and MUST NOT invent a source edit. Its
authority MUST come from the locator-only `authenticated_final_disposition_locator`
and a live reread of exact-head all-success CI plus a complete, resolved thread
inventory. Ordinary PR, ownership, tests, LOC, connector-review, CI, and merge
gates remain active; synthetic `PASS`/`UNOBSERVABLE` and a fourth profile review
are forbidden.

Escalation may only move to a strictly higher profile. The executable profile
contract is maintained by the packaged runtime validator.
