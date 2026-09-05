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
in-scope contract/root repair, or an authenticated external finding discovered
on the clean delta-PASS head; the issue-wide terminal limit remains three.

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

The external-finding reason MUST carry a
`codexy.review-control-external-finding.v1` envelope captured by authenticated
GitHub GraphQL. The envelope binds its repository, authenticated owning issue,
source PR, immutable review-thread and comment identities with the canonical
discussion URL, author, observed commit, unique finding IDs, and
repository-relative affected paths. The producer derives
`qualifying_change.finding_ids` from that source; the transition requires its
`observedCommit` to equal the prior delta head and the repair diff to touch every
recorded path. A source with different repository, issue, PR, head, finding set,
or paths is rejected. The source PR's owning issue is provenance and does not
replace the target `reviewControl.issue_number`.

Escalation may only move to a strictly higher profile. The executable profile
contract is maintained by the packaged runtime validator.
