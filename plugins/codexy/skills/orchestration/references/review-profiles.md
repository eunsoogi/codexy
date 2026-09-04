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
from the direct ordered terminal history, for mandatory base integration or an
in-scope contract/root repair, and the issue-wide terminal limit remains three.

Reviewer-backed transitions use authenticated current and previous PR snapshots
from the canonical GitHub readback producer. Snapshots bind the same repository,
PR number, URL, base branch, capture provenance, `baseRefOid`, and `headRefOid`;
the previous snapshot's `reviewControl` is the only predecessor authority.
`previous_control_state` is rejected. Base integration must change and prove
base ancestry. Contract/root repair must retain the base, follow a prior `BLOCK`
delta with findings, bind `qualifying_change.finding_ids` exactly to those
findings, and show the evidence diff changes every finding's recorded path. The
current snapshot's head and base identity are preserved.

Escalation may only move to a strictly higher profile. The executable profile
contract is maintained by the packaged runtime validator.
