use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Result, bail};
use serde::Deserialize;

use super::policy;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Economics {
    schema: String,
    lanes: Vec<Lane>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Lane {
    id: String,
    kind: String,
    profile: String,
    implementation_ms: u64,
    verification_ms: u64,
    review_ms: u64,
    repair_ms: u64,
    full_review_count: u8,
    delta_recheck_count: u8,
    unique_blockers: u32,
    reopened_blockers: u32,
    follow_ups: u32,
    baseline_p0: u32,
    observed_p0: u32,
    baseline_p1: u32,
    observed_p1: u32,
    tokens: Option<u64>,
    token_source: Option<String>,
    review_share_ppm: u32,
}

pub(super) fn check(plugin_root: &std::path::Path, text: &str) -> Result<()> {
    let report: Economics = serde_json::from_str(text)?;
    let profiles = policy::load(plugin_root)?;
    if report.schema != "codexy.review-economics.v1" || report.lanes.is_empty() {
        bail!("review economics must use the closed v1 schema with lanes");
    }
    let ids = report
        .lanes
        .iter()
        .map(|lane| lane.id.as_str())
        .collect::<BTreeSet<_>>();
    let kinds = report
        .lanes
        .iter()
        .map(|lane| lane.kind.as_str())
        .collect::<BTreeSet<_>>();
    if ids.len() != report.lanes.len()
        || !["tiny", "standard", "review_response", "security", "release"]
            .iter()
            .all(|kind| kinds.contains(kind))
    {
        bail!("review economics must measure each representative lane exactly once");
    }
    if !report.lanes.iter().any(|lane| lane.baseline_p0 > 0)
        || !report.lanes.iter().any(|lane| lane.baseline_p1 > 0)
    {
        bail!("review economics must retain seeded P0 safety and P1 correctness parity");
    }
    let mut ratios = BTreeMap::<&str, Vec<f64>>::new();
    for lane in &report.lanes {
        let _ = lane.follow_ups;
        let profile = profiles
            .get(&lane.profile)
            .ok_or_else(|| anyhow::anyhow!("economics names an unknown profile"))?;
        if lane.id.is_empty()
            || lane.implementation_ms == 0
            || lane.baseline_p0 != lane.observed_p0
            || lane.baseline_p1 != lane.observed_p1
            || lane.full_review_count > profile.full_review_limit
            || lane.delta_recheck_count > profile.delta_recheck_limit
            || lane.reopened_blockers > lane.unique_blockers
            || lane.tokens.is_some_and(|tokens| tokens == 0)
            || (lane.tokens.is_some() != (lane.token_source.as_deref() == Some("runtime")))
            || lane.review_share_ppm != review_share(lane)
        {
            bail!("review economics violates parity, budget, or measured-time invariants");
        }
        if matches!(lane.profile.as_str(), "standard" | "strict") {
            ratios
                .entry(&lane.profile)
                .or_default()
                .push(lane.review_ms as f64 / lane.implementation_ms as f64);
        }
    }
    if median(ratios.get("standard")) > 0.30 || median(ratios.get("strict")) > 0.50 {
        bail!("review economics exceeds the profile review-time median budget");
    }
    Ok(())
}

fn median(values: Option<&Vec<f64>>) -> f64 {
    let Some(values) = values.filter(|items| !items.is_empty()) else {
        return f64::INFINITY;
    };
    let mut values = values.clone();
    values.sort_by(f64::total_cmp);
    let middle = values.len() / 2;
    if values.len() % 2 == 0 {
        (values[middle - 1] + values[middle]) / 2.0
    } else {
        values[middle]
    }
}

fn review_share(lane: &Lane) -> u32 {
    let total = u128::from(lane.implementation_ms)
        + u128::from(lane.verification_ms)
        + u128::from(lane.review_ms)
        + u128::from(lane.repair_ms);
    u32::try_from(u128::from(lane.review_ms) * 1_000_000 / total).unwrap_or(u32::MAX)
}
