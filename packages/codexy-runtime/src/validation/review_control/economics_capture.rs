use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use anyhow::{Result, bail};
use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};

use super::{
    economics,
    economics_package::{self, LaneSpec, Package},
    policy, repository,
};

#[rustfmt::skip]
pub(super) fn capture(plugin_root: &Path, repository_root: &Path, observer_command: &Path, trusted_receipt: &Path, output: &Path) -> Result<()> {
    let package = economics_package::load(plugin_root)?;
    let head = repository::current_head(repository_root)?;
    let tree = git(repository_root, ["rev-parse", "HEAD^{tree}"])?;
    let profiles = policy::load(plugin_root)?;
    let policy_path = plugin_root.join("skills/orchestration/references/review-profiles.json");
    let policy_sha256 = digest(&fs::read(&policy_path)?);
    let receipt_path = external(trusted_receipt, repository_root)?;
    let receipt_bytes = fs::read(&receipt_path)?;
    let receipt_sha256 = digest(&receipt_bytes);
    let receipt: Value = serde_json::from_slice(&receipt_bytes)?;
    validate_receipt(&receipt, &package, &profiles, &head, &tree, &policy_sha256)?;
    let observer = external(observer_command, repository_root)?;
    let observer_sha256 = digest(&fs::read(&observer)?);
    let output_path = external(output, repository_root)?;
    let sidecars = external(&output_path.parent().ok_or_else(|| anyhow::anyhow!("capture output has no parent"))?.join("review-economics-observations"), repository_root)?;
    fs::create_dir_all(&sidecars)?;
    let mut lanes = Vec::new();
    for spec in &package.lanes {
        let trusted = receipt["lanes"].as_array().and_then(|items| items.iter().find(|lane| lane["id"] == spec.id)).ok_or_else(|| anyhow::anyhow!("trusted receipt lane is missing"))?;
        let nonce = text(trusted, "nonce")?;
        let started = Instant::now();
        let (mode, reviewer) = behavior(spec, &profiles)?;
        let mut command = Command::new(&observer);
        command.args(["--lane-id", &spec.id, "--profile", &spec.profile, "--execution-mode", mode, "--nonce", nonce, "--input"]).arg(&spec.path);
        if let Some(reviewer) = reviewer { command.args(["--reviewer", &reviewer.name, "--model", &reviewer.model, "--reasoning-effort", &reviewer.reasoning_effort]); }
        let output_value = command.output()?;
        if !output_value.status.success() { bail!("review economics capture command failed for lane {}", spec.id); }
        let ack: Value = serde_json::from_slice(&output_value.stdout)?;
        if ack["schema"] != "codexy.review-economics-capture.v1" || ack["lane_id"] != spec.id || ack["nonce"] != nonce || ack["synthetic"] == true { bail!("review economics capture acknowledgement is missing or synthetic"); }
        let stdout_path = sidecars.join(format!("{}.json", spec.id));
        fs::write(&stdout_path, &output_value.stdout)?;
        let capture_ms = started.elapsed().as_millis().max(1) as u64;
        lanes.push(report_lane(spec, trusted, &package, &receipt_path, &receipt_sha256, &stdout_path, capture_ms, &head)?);
    }
    let report = json!({
        "schema":"codexy.review-economics.v2", "status":"observed", "head_oid":head, "tree_oid":tree,
        "policy_sha256":policy_sha256, "corpus_sha256":package.corpus_sha256, "package_sha256":package.manifest_sha256,
        "baseline_sha256":package.baseline_sha256, "reason":null,
        "provenance":{"schema":"codexy.review-economics-provenance.v1","runner":"codexy-review-control","source":"codex-app-trusted-receipt","authenticated":true,"synthetic":false,"captured_at_unix_ms":now_ms(),"observer_command_sha256":observer_sha256,"trusted_receipt_sha256":receipt_sha256,"unavailable_fields":["tokens","runtime_telemetry"]},
        "lanes":lanes
    });
    economics::check(plugin_root, repository_root, &serde_json::to_string(&report)?)?;
    fs::write(output_path, serde_json::to_vec_pretty(&report)?)?;
    Ok(())
}

#[rustfmt::skip]
fn behavior<'a>(spec: &LaneSpec, profiles: &'a std::collections::BTreeMap<String, policy::Profile>) -> Result<(&'static str, Option<&'a policy::Reviewer>)> {
    let profile = profiles.get(&spec.profile).ok_or_else(|| anyhow::anyhow!("capture lane names an unknown profile"))?;
    Ok(match spec.profile.as_str() { "light" => ("no-llm", None), "standard" => ("inspector", profile.reviewer.as_ref()), "strict" => ("sentinel", profile.reviewer.as_ref()), _ => return Err(anyhow::anyhow!("capture lane profile is outside the closed policy")) })
}

#[rustfmt::skip]
fn validate_receipt(receipt: &Value, package: &Package, profiles: &std::collections::BTreeMap<String, policy::Profile>, head: &str, tree: &str, policy_sha256: &str) -> Result<()> {
    let execution = &receipt["execution_receipt"];
    if receipt["schema"] != "codexy.codex-observation-receipt.v1" || receipt["source"] != "codex-app"
        || receipt["head_oid"] != head || receipt["tree_oid"] != tree || receipt["policy_sha256"] != policy_sha256
        || receipt["corpus_sha256"] != package.corpus_sha256 || receipt["package_sha256"] != package.manifest_sha256
        || receipt["baseline_sha256"] != package.baseline_sha256 || execution["schema"] != "codexy.codex-task-tool-receipt.v1"
        || execution["authority"] != "codex-app" || execution["authenticated"] != true
        || text(execution, "receipt_id").is_err() || text(execution, "task_id").is_err() || text(execution, "tool_call_id").is_err()
        || text(execution, "tool_name").is_err() || text(execution, "attestation").is_err()
        || receipt["lanes"].as_array().map_or(true, |items| items.len() != package.lanes.len())
    { bail!("trusted Codex task/tool receipt is absent, stale, or not inspectable"); }
    for spec in &package.lanes {
        let lane = receipt["lanes"].as_array().unwrap().iter().find(|item| item["id"] == spec.id).ok_or_else(|| anyhow::anyhow!("trusted receipt does not cover every lane"))?;
        let expected = profiles.get(&spec.profile).and_then(|profile| profile.reviewer.as_ref()).map(|reviewer| json!({"name":reviewer.name,"model":reviewer.model,"reasoning_effort":reviewer.reasoning_effort})).unwrap_or(Value::Null);
        if lane["profile"] != spec.profile || lane["reviewer"] != expected || lane["input_sha256"] != spec.sha256 || text(lane, "nonce").is_err() || text(lane, "task_id").is_err() || text(lane, "tool_call_id").is_err() { bail!("trusted receipt lane is not bound to the package and profile"); }
    }
    Ok(())
}

#[rustfmt::skip]
fn report_lane(spec: &LaneSpec, trusted: &Value, package: &Package, receipt_path: &Path, receipt_sha256: &str, stdout_path: &Path, capture_ms: u64, head: &str) -> Result<Value> {
    let outcomes = trusted["seed_outcomes"].clone();
    let p0 = outcomes.as_array().unwrap().iter().filter(|seed| seed["severity"] == "p0" && seed["detected"] == true).count();
    let p1 = outcomes.as_array().unwrap().iter().filter(|seed| seed["severity"] == "p1" && seed["detected"] == true).count();
    let mut artifacts = vec![json!({"kind":"lane-input","id":spec.id,"path":spec.path,"sha256":spec.sha256})];
    for seed_id in &spec.seed_ids { let seed = package.seeds.get(seed_id).ok_or_else(|| anyhow::anyhow!("lane seed is missing"))?; artifacts.push(json!({"kind":"seed","id":seed_id,"path":seed.path,"sha256":seed.sha256})); }
    artifacts.push(json!({"kind":"observer-output","id":spec.id,"path":stdout_path,"sha256":digest(&fs::read(stdout_path)?)}));
    artifacts.push(json!({"kind":"trusted-receipt","id":spec.id,"path":receipt_path,"sha256":receipt_sha256}));
    Ok(json!({"id":spec.id,"kind":spec.kind,"profile":spec.profile,"head_oid":head,"outcome":trusted["outcome"],"reviewer":trusted["reviewer"],"implementation_ms":trusted["timing"]["implementation_ms"],"verification_ms":trusted["timing"]["verification_ms"],"review_ms":trusted["timing"]["review_ms"],"repair_ms":trusted["timing"]["repair_ms"],"full_review_count":trusted["cycles"]["full_review"],"delta_recheck_count":trusted["cycles"]["delta_recheck"],"unique_blockers":trusted["unique_blockers"],"reopened_blockers":trusted["reopened_blockers"],"follow_ups":trusted["follow_ups"],"baseline_p0":spec.seed_ids.iter().filter(|id| package.seeds.get(*id).is_some_and(|seed| seed.severity == "p0")).count(),"observed_p0":p0,"baseline_p1":spec.seed_ids.iter().filter(|id| package.seeds.get(*id).is_some_and(|seed| seed.severity == "p1")).count(),"observed_p1":p1,"tokens":null,"token_source":null,"cost":null,"cost_source":null,"seed_outcomes":outcomes,"telemetry":trusted["telemetry"],"artifacts":artifacts,"observation":{"schema":"codexy.review-economics-observation.v1","authenticated":true,"synthetic":false,"source":"codex-capture","capture_ms":capture_ms,"stdout_path":stdout_path,"stdout_sha256":digest(&fs::read(stdout_path)?),"receipt_path":receipt_path,"receipt_sha256":receipt_sha256,"nonce_sha256":digest(text(trusted,"nonce")?.as_bytes())}}))
}

#[rustfmt::skip]
fn external(path: &Path, repository_root: &Path) -> Result<PathBuf> { let absolute = if path.is_absolute() { path.to_owned() } else { std::env::current_dir()?.join(path) }; let parent = fs::canonicalize(absolute.parent().ok_or_else(|| anyhow::anyhow!("artifact path has no parent"))?)?; let resolved = if absolute.exists() { fs::canonicalize(&absolute)? } else { parent.join(absolute.file_name().ok_or_else(|| anyhow::anyhow!("artifact path has no name"))?) }; if resolved.starts_with(fs::canonicalize(repository_root)?) { bail!("authentic capture artifacts must be external to the repository"); } Ok(resolved) }
#[rustfmt::skip]
fn text<'a>(value: &'a Value, key: &str) -> Result<&'a str> { value[key].as_str().filter(|text| !text.is_empty()).ok_or_else(|| anyhow::anyhow!("missing receipt field: {key}")) }
#[rustfmt::skip]
fn now_ms() -> u128 { SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() }
#[rustfmt::skip]
fn git<const N: usize>(root: &Path, args: [&str; N]) -> Result<String> { let output = Command::new("git").current_dir(root).args(args).output()?; if !output.status.success() { bail!("authoritative Git identity check failed"); } Ok(String::from_utf8(output.stdout)?.trim().to_owned()) }
#[rustfmt::skip]
fn digest(bytes: &[u8]) -> String { format!("{:x}", Sha256::digest(bytes)) }
