use std::{fs, path::Path};

use anyhow::{Result, bail};

use super::{
    finding_disposition,
    history::{Blocker, Event, History},
    packet::Packet,
    policy::Profile,
};

pub(super) fn record(path: &Path, packet: &Packet, profile: &Profile) -> Result<()> {
    let mut history = if path.exists() {
        serde_json::from_slice(&fs::read(path)?)?
    } else {
        History::new()
    };
    history.validate()?;
    history.events.push(Event {
        id: packet.event_id.clone(),
        predecessor_event_id: super::presence::RequiredNullable::new(
            packet.predecessor_event_id.clone(),
        ),
        profile: packet.profile.clone(),
        base_oid: packet.identity_base().to_owned(),
        head_oid: packet.identity_head().to_owned(),
        state: packet.state.clone(),
        full_used: packet.budget.full_used,
        delta_used: packet.budget.delta_used,
        blockers: packet
            .findings
            .iter()
            .filter(|finding| finding_disposition::is_blocker(finding))
            .map(|finding| Blocker {
                id: finding.id.clone(),
                defect_class: finding.defect_class.clone(),
                resolved: finding.resolved,
                reopen_count: finding.reopen_count,
            })
            .collect(),
        boundaries: packet.boundaries().to_vec(),
        issue_contract: packet.issue_contract().clone(),
        issue_contract_sha256: packet.issue_contract().digest(),
        escalation: super::presence::RequiredNullable::new(packet.escalation().cloned()),
    });
    history.validate()?;
    let event = history
        .events
        .last()
        .ok_or_else(|| anyhow::anyhow!("review event is absent"))?;
    if packet.readiness_budget_exhausted()
        != (event.full_used == profile.full_review_limit
            && event.delta_used == profile.delta_recheck_limit)
    {
        bail!("review packet budget exhaustion must match the typed review cycle");
    }
    fs::write(path, serde_json::to_vec_pretty(&history)?)?;
    Ok(())
}
