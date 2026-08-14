use super::history::Event;

pub(super) fn preserves(prior: &Event, next: &Event) -> bool {
    next.issue_contract == prior.issue_contract
        && next.issue_contract_sha256 == prior.issue_contract_sha256
}
