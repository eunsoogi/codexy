use std::{collections::BTreeSet, path::Path};

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

use super::{
    ledger,
    policy::{self, Reviewer},
    repository,
};

#[path = "packet_readiness.rs"]
mod readiness;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Packet {
    pub(super) schema: String,
    pub(super) event_id: String,
    pub(super) predecessor_event_id: Option<String>,
    pub(super) profile: String,
    pub(super) state: String,
    pub(super) reviewer: Option<Reviewer>,
    identity: Identity,
    acceptance_criteria: Vec<Criterion>,
    changed_files: Vec<String>,
    direct_boundaries: Vec<String>,
    verification_results: Vec<Verification>,
    pub(super) findings: Vec<Finding>,
    resolution: Resolution,
    pub(super) budget: Budget,
    readiness_export: ReadinessExport,
    escalation: Option<Escalation>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Identity {
    base_oid: String,
    head_oid: String,
    diff_sha256: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Criterion {
    id: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Verification {
    id: String,
    head_oid: String,
    evidence_path: String,
    evidence_sha256: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Finding {
    pub(super) id: String,
    pub(super) defect_class: String,
    criterion_id: String,
    counterexample: String,
    head_oid: String,
    pub(super) kind: String,
    pub(super) reopen_count: u8,
    pub(super) resolved: bool,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Resolution {
    repaired_finding_ids: Vec<String>,
    changed_boundaries: Vec<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Budget {
    pub(super) full_used: u8,
    pub(super) delta_used: u8,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReadinessExport {
    head_oid: String,
    profile: String,
    reviewer: Option<Reviewer>,
    unresolved_blocker_ids: Vec<String>,
    budget_exhausted: bool,
    parent_decision_required: bool,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Escalation {
    pub(super) from_profile: String,
    pub(super) predecessor_event_id: String,
    pub(super) discarded_lower_profile: bool,
}

pub(super) fn check(
    plugin_root: &Path,
    repository_root: &Path,
    ledger_path: &Path,
    text: &str,
) -> Result<()> {
    let packet: Packet = serde_json::from_str(text)?;
    let profiles = policy::load(plugin_root)?;
    let profile = profiles
        .get(&packet.profile)
        .ok_or_else(|| anyhow::anyhow!("review packet names an unknown profile"))?;
    let current = repository::Current::load(repository_root, &packet.identity.base_oid)?;
    validate(&packet, profile, repository_root, &current)?;
    ledger::record(ledger_path, &packet, profile)
}

fn validate(
    packet: &Packet,
    profile: &policy::Profile,
    repository_root: &Path,
    current: &repository::Current,
) -> Result<()> {
    if packet.schema != "codexy.review-packet.v2"
        || packet.event_id.is_empty()
        || packet.identity.base_oid != current.base_oid
        || packet.identity.head_oid != current.head_oid
        || packet.identity.diff_sha256 != current.diff_sha256
        || packet.reviewer != profile.reviewer
        || packet.changed_files.iter().collect::<BTreeSet<_>>()
            != current.changed_files.iter().collect()
        || packet.changed_files.len() != current.changed_files.len()
    {
        bail!(
            "review packet must bind the exact current head, diff, selected reviewer, and changed files"
        );
    }
    let criteria = unique(
        packet.acceptance_criteria.iter().map(|item| &item.id),
        "acceptance criterion",
    )?;
    let boundaries = unique(packet.direct_boundaries.iter(), "direct boundary")?;
    let findings = unique(packet.findings.iter().map(|item| &item.id), "finding")?;
    if criteria.is_empty()
        || boundaries.is_empty()
        || current.changed_files.is_empty()
        || packet.verification_results.is_empty()
    {
        bail!(
            "review packet requires current-head acceptance, boundary, change, and verification evidence"
        );
    }
    for result in &packet.verification_results {
        if result.id.is_empty()
            || result.head_oid != current.head_oid
            || repository::blob_digest(repository_root, &current.head_oid, &result.evidence_path)?
                != result.evidence_sha256
        {
            bail!("verification evidence must be an immutable current-head blob");
        }
    }
    if packet
        .findings
        .iter()
        .filter(|item| item.kind == "blocker")
        .count()
        > usize::from(profile.max_blocking_findings)
        || packet.findings.iter().any(|item| {
            !matches!(
                item.kind.as_str(),
                "blocker" | "evidence_only" | "github_metadata" | "follow_up"
            ) || item.defect_class.is_empty()
                || item.counterexample.is_empty()
                || item.head_oid != current.head_oid
                || !criteria.contains(item.criterion_id.as_str())
                || item.reopen_count > 2
        })
    {
        bail!("review packet finding is not a current-head in-scope bounded finding");
    }
    readiness::validate(packet, &findings, &boundaries, &current.head_oid)?;
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
impl Packet {
    pub(super) fn identity_head(&self) -> &str {
        &self.identity.head_oid
    }
    pub(super) fn boundaries(&self) -> &[String] {
        &self.direct_boundaries
    }
    pub(super) const fn readiness_budget_exhausted(&self) -> bool {
        self.readiness_export.budget_exhausted
    }
    pub(super) fn escalation(&self) -> Option<&Escalation> {
        self.escalation.as_ref()
    }
}
