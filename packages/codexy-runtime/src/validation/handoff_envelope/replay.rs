use anyhow::{Result, ensure};
use std::collections::BTreeSet;

use super::{HandoffAuthority, HandoffEnvelope, parse_envelope, validate_intrinsic};

pub(super) fn validate_single(text: &str, authority: &HandoffAuthority) -> Result<HandoffEnvelope> {
    let mut candidate = authority.seen_event_ids.borrow().clone();
    let envelope = validate_one(text, authority, &mut candidate)?;
    publish(authority, candidate);
    Ok(envelope)
}

pub(super) fn validate_batch(
    texts: &[&str],
    authority: &HandoffAuthority,
) -> Result<Vec<HandoffEnvelope>> {
    let mut candidate = authority.seen_event_ids.borrow().clone();
    let mut envelopes = Vec::with_capacity(texts.len());
    for text in texts {
        envelopes.push(validate_one(text, authority, &mut candidate)?);
    }
    publish(authority, candidate);
    Ok(envelopes)
}

fn validate_one(
    text: &str,
    authority: &HandoffAuthority,
    seen_event_ids: &mut BTreeSet<String>,
) -> Result<HandoffEnvelope> {
    let envelope = parse_envelope(text, authority.stable.as_ref())?;
    validate_intrinsic(&envelope)?;
    let volatile = &envelope.volatile;
    ensure!(
        volatile.base_head_sha.head == authority.current_head,
        "stale HEAD"
    );
    ensure!(
        volatile.owner_worktree.owner == authority.owner
            && volatile.owner_worktree.worktree == authority.worktree,
        "owner/worktree authority"
    );
    let (identity, branch, base) = &authority.lane;
    ensure!(
        volatile.issue_pr_identity == *identity
            && volatile.owner_worktree.branch == *branch
            && volatile.base_head_sha.base == *base,
        "lane authority"
    );
    ensure!(
        seen_event_ids.insert(volatile.event.id.clone()),
        "handoff event is a duplicate"
    );
    Ok(envelope)
}

fn publish(authority: &HandoffAuthority, candidate: BTreeSet<String>) {
    *authority.seen_event_ids.borrow_mut() = candidate;
}
