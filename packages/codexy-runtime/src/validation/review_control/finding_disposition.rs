use std::collections::BTreeSet;

use super::packet::Finding;

pub(super) const IN_SCOPE_BLOCKER: &str = "in_scope_blocker";

pub(super) fn is_blocker(finding: &Finding) -> bool {
    finding.disposition == IN_SCOPE_BLOCKER
}

pub(super) fn is_valid(
    finding: &Finding,
    criteria: &BTreeSet<&str>,
    boundaries: &BTreeSet<&str>,
    head_oid: &str,
) -> bool {
    if finding.defect_class.is_empty()
        || finding.counterexample.is_empty()
        || finding.head_oid != head_oid
        || finding.reopen_count > 2
    {
        return false;
    }
    match finding.disposition.as_str() {
        IN_SCOPE_BLOCKER | "in_scope_nonblocking" => {
            cites_owned_rule(finding, criteria)
                && names_boundary(&finding.owned_boundary, boundaries)
                && names_boundary(&finding.repair_boundary, boundaries)
        }
        "out_of_scope_followup" | "rejected" => {
            finding.criterion_id.is_none()
                && finding.owned_invariant.is_none()
                && finding.owned_boundary.is_none()
                && finding.repair_boundary.is_none()
        }
        _ => false,
    }
}

fn cites_owned_rule(finding: &Finding, criteria: &BTreeSet<&str>) -> bool {
    finding
        .criterion_id
        .as_deref()
        .is_some_and(|criterion| criteria.contains(criterion))
        || finding
            .owned_invariant
            .as_deref()
            .is_some_and(|invariant| !invariant.is_empty())
}

fn names_boundary(boundary: &Option<String>, boundaries: &BTreeSet<&str>) -> bool {
    boundary
        .as_deref()
        .is_some_and(|boundary| boundaries.contains(boundary))
}
