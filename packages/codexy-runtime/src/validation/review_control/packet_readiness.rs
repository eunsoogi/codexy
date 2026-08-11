use std::collections::BTreeSet;

use anyhow::{Result, bail};

use super::Packet;

pub(super) fn validate(
    packet: &Packet,
    findings: &BTreeSet<&str>,
    boundaries: &BTreeSet<&str>,
    head_oid: &str,
) -> Result<()> {
    let resolved = packet
        .findings
        .iter()
        .filter(|finding| finding.resolved)
        .map(|finding| finding.id.as_str())
        .collect::<BTreeSet<_>>();
    let repaired = unique(
        packet.resolution.repaired_finding_ids.iter(),
        "repaired finding",
    )?;
    if repaired != resolved
        || (!resolved.is_empty() && packet.resolution.changed_boundaries.is_empty())
        || packet.resolution.repaired_finding_ids.iter().any(|id| {
            !findings.contains(id.as_str())
                || !packet
                    .findings
                    .iter()
                    .any(|finding| finding.id == *id && finding.resolved)
        })
        || packet
            .resolution
            .changed_boundaries
            .iter()
            .any(|id| !boundaries.contains(id.as_str()))
    {
        bail!("review packet resolution must name resolved findings and direct boundaries");
    }
    let unresolved = packet
        .findings
        .iter()
        .filter(|finding| finding.kind == "blocker" && !finding.resolved)
        .map(|finding| finding.id.as_str())
        .collect::<BTreeSet<_>>();
    if unique(
        packet.readiness_export.unresolved_blocker_ids.iter(),
        "unresolved blocker",
    )?
    .iter()
    .copied()
    .collect::<BTreeSet<_>>()
        != unresolved
        || packet.readiness_export.head_oid != head_oid
        || packet.readiness_export.profile != packet.profile
        || packet.readiness_export.reviewer != packet.reviewer
        || packet.readiness_export.parent_decision_required
            != matches!(packet.state.as_str(), "parent_decision" | "unobservable")
    {
        bail!("review packet readiness export must be a packet-bound policy-free summary");
    }
    Ok(())
}

fn unique<'a>(items: impl Iterator<Item = &'a String>, label: &str) -> Result<BTreeSet<&'a str>> {
    let values = items.map(String::as_str).collect::<Vec<_>>();
    let unique = values.iter().copied().collect::<BTreeSet<_>>();
    if values.len() != unique.len() || unique.iter().any(|value| value.is_empty()) {
        bail!("review packet {label} values must be unique and non-empty");
    }
    Ok(unique)
}
