use std::collections::BTreeSet;

use super::history::{Blocker, Event};

pub(super) fn valid(event: &Event) -> bool {
    valid_id(&event.id)
        && valid_id(&event.base_oid)
        && valid_id(&event.head_oid)
        && valid_boundaries(&event.boundaries)
        && valid_blockers(&event.blockers)
        && event.issue_contract.authority().is_ok()
        && event.issue_contract_sha256 == event.issue_contract.digest()
}

fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

fn valid_boundaries(boundaries: &[String]) -> bool {
    !boundaries.is_empty()
        && boundaries.iter().all(|boundary| !boundary.is_empty())
        && boundaries.iter().collect::<BTreeSet<_>>().len() == boundaries.len()
}

fn valid_blockers(blockers: &[Blocker]) -> bool {
    blockers
        .iter()
        .all(|blocker| valid_id(&blocker.id) && !blocker.defect_class.is_empty())
        && blockers
            .iter()
            .map(|blocker| blocker.id.as_str())
            .collect::<BTreeSet<_>>()
            .len()
            == blockers.len()
}
