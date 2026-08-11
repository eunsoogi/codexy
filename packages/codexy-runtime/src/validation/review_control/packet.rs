use std::collections::BTreeSet;

use anyhow::{Result, bail};
use serde::Deserialize;

use super::policy::{self, Reviewer};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Packet {
    schema: String,
    profile: String,
    state: String,
    reviewer: Option<Reviewer>,
    identity: Identity,
    acceptance_criteria: Vec<Criterion>,
    changed_files: Vec<String>,
    direct_boundaries: Vec<String>,
    verification_results: Vec<Verification>,
    findings: Vec<Finding>,
    resolution: Resolution,
    budget: Budget,
    readiness_export: ReadinessExport,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Identity {
    base_sha: String,
    head_sha: String,
    diff_sha: String,
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
    head_sha: String,
    passed: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Finding {
    id: String,
    defect_class: String,
    criterion_id: String,
    counterexample: String,
    head_sha: String,
    kind: String,
    reopen_count: u8,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Resolution {
    repaired_finding_ids: Vec<String>,
    changed_boundaries: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Budget {
    full_used: u8,
    delta_used: u8,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReadinessExport {
    head_sha: String,
    profile: String,
    reviewer: Option<Reviewer>,
    unresolved_blocker_ids: Vec<String>,
    budget_exhausted: bool,
    parent_decision_required: bool,
}

pub(super) fn check(plugin_root: &std::path::Path, text: &str) -> Result<()> {
    let packet: Packet = serde_json::from_str(text)?;
    let profiles = policy::load(plugin_root)?;
    let profile = profiles
        .get(&packet.profile)
        .ok_or_else(|| anyhow::anyhow!("review packet names an unknown profile"))?;
    validate(&packet, profile)
}

fn validate(packet: &Packet, profile: &policy::Profile) -> Result<()> {
    if packet.schema != "codexy.review-packet.v1"
        || !sha(&packet.identity.base_sha)
        || !sha(&packet.identity.head_sha)
        || !sha(&packet.identity.diff_sha)
    {
        bail!("review packet must bind base, head, and diff to 64-character SHA identities");
    }
    if packet.reviewer != profile.reviewer
        || packet.budget.full_used > profile.full_review_limit
        || packet.budget.delta_used > profile.delta_recheck_limit
    {
        bail!("review packet reviewer or budget violates the selected profile");
    }
    let criteria = unique(
        packet.acceptance_criteria.iter().map(|item| &item.id),
        "acceptance criterion",
    )?;
    let boundaries = unique(packet.direct_boundaries.iter(), "direct boundary")?;
    unique(packet.changed_files.iter(), "changed file")?;
    let findings = unique(packet.findings.iter().map(|item| &item.id), "finding")?;
    unique(
        packet.verification_results.iter().map(|item| &item.id),
        "verification result",
    )?;
    if criteria.is_empty()
        || boundaries.is_empty()
        || packet.changed_files.is_empty()
        || packet.verification_results.is_empty()
        || packet
            .verification_results
            .iter()
            .any(|item| !item.passed || item.head_sha != packet.identity.head_sha)
    {
        bail!(
            "review packet requires current-head acceptance, boundary, file, and verification evidence"
        );
    }
    let blockers = packet
        .findings
        .iter()
        .filter(|item| item.kind == "blocker")
        .count();
    if blockers > usize::from(profile.max_blocking_findings)
        || packet.findings.iter().any(|item| {
            !matches!(
                item.kind.as_str(),
                "blocker" | "evidence_only" | "github_metadata" | "follow_up"
            ) || item.defect_class.is_empty()
                || item.counterexample.is_empty()
                || item.head_sha != packet.identity.head_sha
                || !criteria.contains(item.criterion_id.as_str())
                || item.reopen_count > 2
        })
    {
        bail!("review packet finding is not a current-head in-scope bounded finding");
    }
    readiness(packet, &findings)?;
    state(packet, profile, &findings, &boundaries)
}

fn readiness(packet: &Packet, findings: &BTreeSet<&str>) -> Result<()> {
    let export = &packet.readiness_export;
    if export.head_sha != packet.identity.head_sha
        || export.profile != packet.profile
        || export.reviewer != packet.reviewer
        || unique(export.unresolved_blocker_ids.iter(), "unresolved blocker")?
            .iter()
            .any(|id| {
                !findings.contains(id)
                    || !packet
                        .findings
                        .iter()
                        .any(|finding| finding.id == *id && finding.kind == "blocker")
            })
        || export.budget_exhausted
            != (packet.budget.full_used == 1 && packet.budget.delta_used == 1)
        || export.parent_decision_required != (packet.state == "parent_decision")
    {
        bail!("review packet readiness export must be a packet-bound policy-free summary");
    }
    Ok(())
}

fn state(
    packet: &Packet,
    profile: &policy::Profile,
    findings: &BTreeSet<&str>,
    boundaries: &BTreeSet<&str>,
) -> Result<()> {
    if profile.reviewer.is_none() {
        if packet.state == "passed" && packet.budget.full_used == 0 && packet.budget.delta_used == 0
        {
            return Ok(());
        }
        bail!("light review packets must terminate without an LLM reviewer");
    }
    let delta =
        packet.state == "delta" && packet.budget.full_used == 1 && packet.budget.delta_used == 1;
    let full =
        packet.state == "full" && packet.budget.full_used == 1 && packet.budget.delta_used == 0;
    let terminal = matches!(
        packet.state.as_str(),
        "passed" | "unobservable" | "parent_decision"
    ) && packet.budget.full_used == 1;
    if !(delta || full || terminal) {
        bail!("review packet state must be a bounded full, delta, or terminal transition");
    }
    if delta
        && (packet.resolution.repaired_finding_ids.is_empty()
            || packet.resolution.changed_boundaries.is_empty()
            || packet
                .resolution
                .repaired_finding_ids
                .iter()
                .any(|id| !findings.contains(id.as_str()))
            || packet
                .resolution
                .changed_boundaries
                .iter()
                .any(|id| !boundaries.contains(id.as_str())))
    {
        bail!("delta recheck requires repairs to named findings and direct boundaries");
    }
    if packet.findings.iter().any(|item| item.reopen_count == 1) && !delta {
        bail!("a reopened finding consumes the one permitted delta recheck");
    }
    if packet.findings.iter().any(|item| item.reopen_count == 2)
        && !(packet.state == "parent_decision" && packet.budget.delta_used == 1)
    {
        bail!("a second recurrence requires a parent decision without a third review");
    }
    Ok(())
}

fn unique<'a>(items: impl Iterator<Item = &'a String>, label: &str) -> Result<BTreeSet<&'a str>> {
    let items = items.map(String::as_str).collect::<Vec<_>>();
    let values = items.iter().copied().collect::<BTreeSet<_>>();
    if values.len() != items.len() || values.iter().any(|item| item.is_empty()) {
        bail!("review packet {label} values must be unique and non-empty");
    }
    Ok(values)
}

fn sha(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}
