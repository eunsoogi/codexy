use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
    process::Command,
};

use anyhow::{Result, bail};
use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};

use super::{economics_package, policy, repository};

#[rustfmt::skip]
pub(super) fn check(plugin_root: &Path, repository_root: &Path, input: &str) -> Result<()> {
    let report: Value = serde_json::from_str(input)?;
    if report["status"] == "unavailable" { validate_unavailable(&report)?; bail!("review economics observations are unavailable and cannot prove profile economics"); }
    let package = economics_package::load(plugin_root)?;
    let head = repository::current_head(repository_root)?;
    let tree = git(repository_root, ["rev-parse", "HEAD^{tree}"])?;
    let policy_path = plugin_root.join("skills/orchestration/references/review-profiles.json");
    let policy_digest = digest(&fs::read(&policy_path)?);
    if report["schema"] != "codexy.review-economics.v2" || report["status"] != "observed"
        || report["head_oid"] != head || report["tree_oid"] != tree || report["policy_sha256"] != policy_digest
        || report["corpus_sha256"] != package.corpus_sha256 || report["package_sha256"] != package.manifest_sha256
        || report["baseline_sha256"] != package.baseline_sha256 || !report["reason"].is_null()
    { bail!("review economics must bind observed inputs to the current head and package"); }
    provenance(&report)?;
    let profiles = policy::load(plugin_root)?;
    let receipt = receipt(&report, repository_root, &head, &tree, &policy_digest, &package)?;
    let lanes = report["lanes"].as_array().ok_or_else(|| anyhow::anyhow!("economics lanes must be an array"))?;
    let expected = package.lanes.iter().map(|lane| lane.id.as_str()).collect::<BTreeSet<_>>();
    let actual = lanes.iter().map(|lane| lane["id"].as_str().unwrap_or_default()).collect::<BTreeSet<_>>();
    if lanes.len() != package.lanes.len() || actual != expected { bail!("review economics must measure each representative lane exactly once"); }
    let mut ratios = BTreeMap::<&str, Vec<(u64, u64)>>::new();
    for lane in lanes {
        let id = text(lane, "id")?;
        let spec = package.lanes.iter().find(|item| item.id == id).ok_or_else(|| anyhow::anyhow!("economics lane is outside the package"))?;
        let trusted = receipt["lanes"].as_array().and_then(|items| items.iter().find(|item| item["id"] == id)).ok_or_else(|| anyhow::anyhow!("economics lane is absent from the trusted receipt"))?;
        validate_lane(lane, trusted, spec, &package, &profiles, &head, &tree, repository_root)?;
        if matches!(lane["profile"].as_str(), Some("standard") | Some("strict")) { ratios.entry(text(lane, "profile")?).or_default().push((number(lane, "review_ms")?, number(lane, "implementation_ms")?)); }
    }
    if !within_budget(ratios.get("standard"), 300_000) || !within_budget(ratios.get("strict"), 500_000) { bail!("review economics exceeds the profile review-time median budget"); }
    Ok(())
}

#[rustfmt::skip]
fn validate_unavailable(report: &Value) -> Result<()> {
    let fields = ["head_oid", "tree_oid", "policy_sha256", "corpus_sha256", "package_sha256", "baseline_sha256", "provenance"];
    if report["schema"] != "codexy.review-economics.v2" || fields.iter().any(|field| !report[*field].is_null())
        || !report["lanes"].as_array().is_some_and(Vec::is_empty) || text(report, "reason").is_err()
    { bail!("unavailable review economics must record null measurements without an acceptance claim"); }
    Ok(())
}

#[rustfmt::skip]
fn provenance(report: &Value) -> Result<()> {
    let value = &report["provenance"];
    let unavailable = value["unavailable_fields"].as_array().ok_or_else(|| anyhow::anyhow!("provenance unavailable fields are missing"))?;
    let expected = ["tokens", "runtime_telemetry"];
    if value["schema"] != "codexy.review-economics-provenance.v1" || value["runner"] != "codexy-review-control"
        || value["source"] != "codex-app-trusted-receipt" || value["authenticated"] != true || value["synthetic"] != false
        || number(value, "captured_at_unix_ms")? == 0 || text(value, "observer_command_sha256").is_err()
        || text(value, "trusted_receipt_sha256").is_err() || unavailable.len() != expected.len()
        || unavailable.iter().zip(expected).any(|(got, want)| got.as_str() != Some(want))
    { bail!("review economics provenance is missing trusted execution evidence"); }
    Ok(())
}

#[rustfmt::skip]
fn receipt(report: &Value, repository_root: &Path, head: &str, tree: &str, policy: &str, package: &economics_package::Package) -> Result<Value> {
    let lanes = report["lanes"].as_array().ok_or_else(|| anyhow::anyhow!("economics lanes are missing"))?;
    let path = text(&lanes.first().ok_or_else(|| anyhow::anyhow!("economics lanes are empty"))?["observation"], "receipt_path")?;
    let canonical = fs::canonicalize(path)?;
    if canonical.starts_with(fs::canonicalize(repository_root)?) { bail!("trusted receipt must be an external Codex execution artifact"); }
    let bytes = fs::read(&canonical)?;
    let hash = digest(&bytes);
    if text(&report["provenance"], "trusted_receipt_sha256")? != hash || lanes.iter().any(|lane| lane["observation"]["receipt_path"] != path || lane["observation"]["receipt_sha256"] != hash) { bail!("review economics trusted receipt binding is stale or forged"); }
    let value: Value = serde_json::from_slice(&bytes)?;
    let execution = &value["execution_receipt"];
    if value["schema"] != "codexy.codex-observation-receipt.v1" || value["source"] != "codex-app"
        || value["head_oid"] != head || value["tree_oid"] != tree || value["policy_sha256"] != policy
        || value["corpus_sha256"] != package.corpus_sha256 || value["package_sha256"] != package.manifest_sha256
        || value["baseline_sha256"] != package.baseline_sha256 || execution["schema"] != "codexy.codex-task-tool-receipt.v1"
        || execution["authority"] != "codex-app" || execution["authenticated"] != true
        || text(execution, "receipt_id").is_err() || text(execution, "task_id").is_err()
        || text(execution, "tool_call_id").is_err() || text(execution, "tool_name").is_err()
        || text(execution, "attestation").is_err() || value["lanes"].as_array().map_or(true, |items| items.len() != package.lanes.len())
    { bail!("trusted Codex task/tool receipt is absent, stale, or not inspectable"); }
    Ok(value)
}

#[rustfmt::skip]
fn validate_lane(lane: &Value, trusted: &Value, spec: &economics_package::LaneSpec, package: &economics_package::Package, profiles: &BTreeMap<String, policy::Profile>, head: &str, tree: &str, repository_root: &Path) -> Result<()> {
    let profile = text(lane, "profile")?;
    let profile_policy = profiles.get(profile).ok_or_else(|| anyhow::anyhow!("economics names an unknown profile"))?;
    let expected_reviewer = profile_policy.reviewer.as_ref().map(|item| json!({"name":item.name,"model":item.model,"reasoning_effort":item.reasoning_effort})).unwrap_or(Value::Null);
    let outcomes = lane["seed_outcomes"].as_array().ok_or_else(|| anyhow::anyhow!("seed outcomes are missing"))?;
    let expected_seeds = spec.seed_ids.iter().map(String::as_str).collect::<BTreeSet<_>>();
    let actual_seeds = outcomes.iter().map(|seed| seed["id"].as_str().unwrap_or_default()).collect::<BTreeSet<_>>();
    let expected_p0 = spec.seed_ids.iter().filter(|id| package.seeds.get(*id).is_some_and(|seed| seed.severity == "p0")).count() as u64;
    let expected_p1 = spec.seed_ids.iter().filter(|id| package.seeds.get(*id).is_some_and(|seed| seed.severity == "p1")).count() as u64;
    let observed_p0 = outcomes.iter().filter(|seed| seed["detected"] == true && seed["severity"] == "p0").count() as u64;
    let observed_p1 = outcomes.iter().filter(|seed| seed["detected"] == true && seed["severity"] == "p1").count() as u64;
    let full = number(lane, "full_review_count")?;
    let delta = number(lane, "delta_recheck_count")?;
    if lane["id"] != spec.id || lane["kind"] != spec.kind || profile != spec.profile || lane["head_oid"] != head
        || lane["reviewer"] != expected_reviewer || text(trusted, "outcome")? != text(lane, "outcome")?
        || trusted["profile"] != profile || trusted["reviewer"] != expected_reviewer || trusted["head_oid"] != head
        || trusted["tree_oid"] != tree || text(trusted, "task_id").is_err() || text(trusted, "tool_call_id").is_err()
        || trusted["input_sha256"] != spec.sha256 || number(lane, "implementation_ms")? != number(trusted, "timing.implementation_ms")?
        || number(lane, "verification_ms")? != number(trusted, "timing.verification_ms")? || number(lane, "review_ms")? != number(trusted, "timing.review_ms")?
        || number(lane, "repair_ms")? != number(trusted, "timing.repair_ms")? || full != number(trusted, "cycles.full_review")?
        || delta != number(trusted, "cycles.delta_recheck")? || number(lane, "unique_blockers")? != number(trusted, "unique_blockers")?
        || number(lane, "reopened_blockers")? != number(trusted, "reopened_blockers")? || number(lane, "follow_ups")? != number(trusted, "follow_ups")?
        || full > u64::from(profile_policy.full_review_limit) || delta > u64::from(profile_policy.delta_recheck_limit)
        || number(lane, "reopened_blockers")? > number(lane, "unique_blockers")? || number(lane, "review_ms")? == 0
        || number(lane, "baseline_p0")? != expected_p0 || number(lane, "observed_p0")? != observed_p0 || observed_p0 != expected_p0
        || number(lane, "baseline_p1")? != expected_p1 || number(lane, "observed_p1")? != observed_p1 || observed_p1 != expected_p1
        || !lane["tokens"].is_null() || !lane["token_source"].is_null() || !lane["cost"].is_null() || !lane["cost_source"].is_null()
        || outcomes.len() != expected_seeds.len() || actual_seeds != expected_seeds
        || outcomes.iter().any(|seed| seed["detected"] != true || seed["required"] != true || package.seeds.get(seed["id"].as_str().unwrap_or_default()).is_none_or(|item| seed["severity"] != item.severity))
    { bail!("review economics violates trusted parity or measured-time invariants"); }
    artifacts(lane, spec, package, repository_root)?;
    observation(lane, trusted, tree)?;
    if lane["telemetry"] != trusted["telemetry"] && !(lane["telemetry"].is_null() && trusted["telemetry"].is_null()) { bail!("runtime telemetry must be copied only when the trusted receipt exposes it"); }
    if !lane["telemetry"].is_null() && lane["telemetry"]["source"] != "runtime" { bail!("runtime telemetry source is not authenticated"); }
    Ok(())
}

#[rustfmt::skip]
fn artifacts(lane: &Value, spec: &economics_package::LaneSpec, package: &economics_package::Package, repository_root: &Path) -> Result<()> {
    let items = lane["artifacts"].as_array().ok_or_else(|| anyhow::anyhow!("lane artifacts are missing"))?;
    let observation_path = text(&lane["observation"], "stdout_path")?;
    let receipt_path = text(&lane["observation"], "receipt_path")?;
    if items.len() != spec.seed_ids.len() + 3 { bail!("review economics artifact set is incomplete"); }
    if fs::canonicalize(observation_path)?.starts_with(fs::canonicalize(repository_root)?) { bail!("observer output must be an external capture artifact"); }
    for item in items {
        let path = text(item, "path")?;
        let bytes = fs::read(path)?;
        if digest(&bytes) != text(item, "sha256")? { bail!("review economics artifact digest is stale or forged"); }
        match text(item, "kind")? {
            "lane-input" if path == spec.path.to_string_lossy() => {}
            "observer-output" if path == observation_path => {}
            "trusted-receipt" if path == receipt_path => {}
            "seed" => { let id = text(item, "id")?; if !package.seeds.iter().any(|(seed_id, seed)| seed_id == id && path == seed.path.to_string_lossy()) { bail!("review economics seed artifact is not an exact package input"); } }
            _ => bail!("review economics artifact is not an exact package input"),
        }
    }
    Ok(())
}

#[rustfmt::skip]
fn observation(lane: &Value, trusted: &Value, tree: &str) -> Result<()> {
    let value = &lane["observation"];
    let bytes = fs::read(text(value, "stdout_path")?)?;
    let ack: Value = serde_json::from_slice(&bytes)?;
    if digest(&bytes) != text(value, "stdout_sha256")? || ack["schema"] != "codexy.review-economics-capture.v1"
        || ack["lane_id"] != lane["id"] || ack["nonce"] != trusted["nonce"] || ack["synthetic"] == true
        || value["schema"] != "codexy.review-economics-observation.v1" || value["authenticated"] != true
        || value["synthetic"] != false || value["source"] != "codex-capture" || number(value, "capture_ms")? == 0
        || text(value, "receipt_path").is_err() || text(value, "receipt_sha256").is_err()
        || text(value, "nonce_sha256")? != digest(text(trusted, "nonce")?.as_bytes()) || tree.is_empty()
    { bail!("review economics observation is not authenticated by a live capture"); }
    Ok(())
}

#[rustfmt::skip]
fn text<'a>(value: &'a Value, path: &str) -> Result<&'a str> { path.split('.').try_fold(value, |current, key| Ok(&current[key])).and_then(|value| value.as_str().filter(|text| !text.is_empty()).ok_or_else(|| anyhow::anyhow!("missing string field: {path}"))) }
#[rustfmt::skip]
fn number(value: &Value, path: &str) -> Result<u64> { path.split('.').try_fold(value, |current, key| Ok(&current[key])).and_then(|value| value.as_u64().ok_or_else(|| anyhow::anyhow!("missing numeric field: {path}"))) }
#[rustfmt::skip]
fn git<const N: usize>(root: &Path, args: [&str; N]) -> Result<String> { let output = Command::new("git").current_dir(root).args(args).output()?; if !output.status.success() { bail!("authoritative Git identity check failed"); } Ok(String::from_utf8(output.stdout)?.trim().to_owned()) }
#[rustfmt::skip]
fn digest(bytes: &[u8]) -> String { format!("{:x}", Sha256::digest(bytes)) }
#[rustfmt::skip]
fn within_budget(values: Option<&Vec<(u64, u64)>>, limit_ppm: u64) -> bool { let Some(values) = values.filter(|items| !items.is_empty()) else { return false; }; let mut values = values.clone(); values.sort_by(|left, right| (u128::from(left.0) * u128::from(right.1)).cmp(&(u128::from(right.0) * u128::from(left.1)))); let middle = values.len() / 2; if values.len() % 2 == 0 { let (a, b) = values[middle - 1]; let (c, d) = values[middle]; (u128::from(a) * u128::from(d) + u128::from(c) * u128::from(b)) * 1_000_000 <= 2 * u128::from(b) * u128::from(d) * u128::from(limit_ppm) } else { u128::from(values[middle].0) * 1_000_000 <= u128::from(values[middle].1) * u128::from(limit_ppm) } }
