use std::{fs, path::Path};

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

use super::{
    packet::{Escalation, Packet},
    policy::{self, Profile},
};

#[derive(Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Ledger {
    schema: String,
    events: Vec<Event>,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Event {
    id: String,
    profile: String,
    head_oid: String,
    state: String,
    full_used: u8,
    delta_used: u8,
    blockers: Vec<Blocker>,
    boundaries: Vec<String>,
    escalation: Option<Escalation>,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Blocker {
    id: String,
    defect_class: String,
    resolved: bool,
    reopen_count: u8,
}

pub(super) fn record(path: &Path, packet: &Packet, profile: &Profile) -> Result<()> {
    let mut ledger = if path.exists() {
        serde_json::from_slice(&fs::read(path)?)?
    } else {
        Ledger {
            schema: "codexy.review-ledger.v1".into(),
            events: Vec::new(),
        }
    };
    if ledger.schema != "codexy.review-ledger.v1"
        || ledger
            .events
            .iter()
            .any(|event| event.id == packet.event_id)
    {
        bail!("review packet event identity is duplicate or ledger schema is invalid");
    }
    let prior = match &packet.predecessor_event_id {
        Some(id) => Some(
            ledger
                .events
                .iter()
                .find(|event| &event.id == id)
                .ok_or_else(|| anyhow::anyhow!("review packet predecessor event is absent"))?,
        ),
        None => None,
    };
    let same = ledger
        .events
        .iter()
        .any(|event| event.head_oid == packet.identity_head());
    let expected = transition(packet, profile, prior, same)?;
    if matches!(packet.state.as_str(), "delta" | "passed") {
        account_for_prior_blockers(packet, prior)?;
    }
    if (packet.budget.full_used, packet.budget.delta_used) != expected
        || packet.readiness_budget_exhausted()
            != (expected.0 == profile.full_review_limit
                && expected.1 == profile.delta_recheck_limit)
    {
        bail!("review packet budget must be derived from its durable predecessor transition");
    }
    ledger.events.push(Event {
        id: packet.event_id.clone(),
        profile: packet.profile.clone(),
        head_oid: packet.identity_head().to_owned(),
        state: packet.state.clone(),
        full_used: expected.0,
        delta_used: expected.1,
        blockers: packet
            .findings
            .iter()
            .filter(|item| item.kind == "blocker")
            .map(|item| Blocker {
                id: item.id.clone(),
                defect_class: item.defect_class.clone(),
                resolved: item.resolved,
                reopen_count: item.reopen_count,
            })
            .collect(),
        boundaries: packet.boundaries().to_vec(),
        escalation: packet.escalation().cloned(),
    });
    fs::write(path, serde_json::to_vec_pretty(&ledger)?)?;
    Ok(())
}

fn account_for_prior_blockers(packet: &Packet, prior: Option<&Event>) -> Result<()> {
    let prior = prior.ok_or_else(|| anyhow::anyhow!("delta has no durable predecessor"))?;
    if prior.blockers.is_empty() || packet.boundaries().is_empty() {
        bail!("delta must account for durable blockers and changed boundaries");
    }
    let prior_ids = prior
        .blockers
        .iter()
        .map(|blocker| blocker.id.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    for old in &prior.blockers {
        let Some(next) = packet.findings.iter().find(|item| item.id == old.id) else {
            bail!("delta omits a prior blocker");
        };
        let expected_reopen_count = if old.resolved && !next.resolved {
            old.reopen_count
                .checked_add(1)
                .ok_or_else(|| anyhow::anyhow!("delta reopens a blocker too many times"))?
        } else {
            old.reopen_count
        };
        if next.kind != "blocker"
            || next.defect_class != old.defect_class
            || next.reopen_count != expected_reopen_count
        {
            bail!("delta mutates durable blocker identity");
        }
    }
    if packet.findings.iter().any(|finding| {
        finding.kind == "blocker" && finding.resolved && !prior_ids.contains(finding.id.as_str())
    }) {
        bail!("delta resolves a blocker outside its durable predecessor");
    }
    Ok(())
}

fn transition(
    packet: &Packet,
    profile: &Profile,
    prior: Option<&Event>,
    same: bool,
) -> Result<(u8, u8)> {
    if packet.escalation().is_some() && packet.state != "full" {
        bail!("review packet escalation is only valid for a replacement full review");
    }
    if profile.reviewer.is_none() {
        if packet.state == "passed" && prior.is_none() && !same {
            return Ok((0, 0));
        }
        bail!("light review packets must terminate without an LLM reviewer");
    }
    match packet.state.as_str() {
        "full" if prior.is_none() && !same && packet.escalation().is_none() => Ok((1, 0)),
        "full"
            if prior.is_some_and(|event| {
                packet.escalation().is_some_and(|escalation| {
                    escalation.discarded_lower_profile
                        && escalation.predecessor_event_id == event.id
                        && escalation.from_profile == event.profile
                        && event.state == "unobservable"
                        && event.head_oid == packet.identity_head()
                        && policy::is_strictly_higher(&event.profile, &packet.profile)
                })
            }) =>
        {
            Ok((1, 0))
        }
        "delta"
            if prior.is_some_and(|event| {
                event.profile == packet.profile
                    && event.state == "full"
                    && event.head_oid == packet.identity_base()
                    && event.head_oid != packet.identity_head()
            }) =>
        {
            Ok((1, 1))
        }
        "passed"
            if prior.is_some_and(|event| {
                event.profile == packet.profile
                    && event.head_oid == packet.identity_head()
                    && matches!(event.state.as_str(), "full" | "delta")
            }) && !packet.has_unresolved_blockers() =>
        {
            let prior = prior.ok_or_else(|| anyhow::anyhow!("passed review has no predecessor"))?;
            Ok((prior.full_used, prior.delta_used))
        }
        "unobservable" if prior.is_none() && !same => Ok((0, 0)),
        "parent_decision"
            if prior.is_some_and(|event| {
                event.profile == packet.profile
                    && event.head_oid == packet.identity_head()
                    && event.state == "delta"
                    && event.delta_used == 1
            }) =>
        {
            Ok((1, 1))
        }
        _ => bail!(
            "review packet transition permits one full review, one same-reviewer delta recheck, then parent decision"
        ),
    }
}
