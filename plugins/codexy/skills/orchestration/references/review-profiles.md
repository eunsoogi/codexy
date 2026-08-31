# Review profiles

The closed review profile set is:

- `light`: no reviewer and no full or delta recheck quota.
- `standard`: one `codexy-inspector` review and one full plus one delta
  recheck.
- `strict`: one `codexy-sentinel` review and one full plus one delta recheck;
  blocking findings remain bounded by the strict contract.

Escalation may only move to a strictly higher profile. The executable profile
contract is maintained by the packaged runtime validator.
