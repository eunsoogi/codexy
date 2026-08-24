use std::{fs, path::{Path, PathBuf}, process::{Command, Output}};

use crate::support::TestResult;
use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};

#[cfg(unix)]
#[test]
fn capture_runner_fails_closed_without_trusted_codex_receipt() -> TestResult {
    let fixture = crate::support::plugin_fixture()?;
    let temp = tempfile::tempdir()?;
    let observer = write_observer(temp.path(), false)?;
    let report = temp.path().join("report.json");
    let output = capture(fixture.root(), &observer, &report, None)?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("--trusted-receipt is required"));
    assert!(!report.exists());
    Ok(())
}

#[cfg(unix)]
#[test]
fn capture_runner_rejects_self_asserted_json_receipt() -> TestResult {
    let fixture = crate::support::plugin_fixture()?;
    let temp = tempfile::tempdir()?;
    let observer = write_observer(temp.path(), false)?;
    let receipt = write_untrusted_receipt(temp.path())?;
    let report = temp.path().join("report.json");
    let output = capture(fixture.root(), &observer, &report, Some(&receipt))?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("no callable verifier"));
    assert!(!report.exists());
    Ok(())
}

#[cfg(unix)]
#[test]
fn fixture_subprocess_can_never_provide_verifiable_observation() -> TestResult {
    let fixture = crate::support::plugin_fixture()?;
    let temp = tempfile::tempdir()?;
    let observer = write_observer(temp.path(), true)?;
    let receipt = write_untrusted_receipt(temp.path())?;
    let report = temp.path().join("report.json");
    let output = capture(fixture.root(), &observer, &report, Some(&receipt))?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("no callable verifier"));
    assert!(!report.exists());
    Ok(())
}

#[cfg(unix)]
#[test]
fn capture_runner_rejects_complete_forged_codex_receipt() -> TestResult {
    let fixture = crate::support::plugin_fixture()?;
    let temp = tempfile::tempdir()?;
    let observer = write_observer(temp.path(), false)?;
    let receipt = write_complete_forged_receipt(fixture.root(), temp.path())?;
    let report = temp.path().join("report.json");
    let output = capture(fixture.root(), &observer, &report, Some(&receipt))?;
    assert!(!output.status.success(), "complete forged receipt was accepted: {}", stderr(&output));
    assert!(stderr(&output).contains("independent") || stderr(&output).contains("platform-issued") || stderr(&output).contains("trusted authority"));
    assert!(!report.exists());
    Ok(())
}

#[cfg(unix)]
fn capture(root: &Path, observer: &Path, output: &Path, receipt: Option<&Path>) -> TestResult<Output> {
    let mut command = Command::new(env!("CARGO_BIN_EXE_codexy-review-control"));
    command.args(["--plugin-root", root.to_str().ok_or("plugin root")?])
        .args(["--repository-root", codexy_runtime::paths::repository_root().to_str().ok_or("repository root")?])
        .args(["--capture-economics", "--observer-command"]).arg(observer).args(["--output"]).arg(output);
    if let Some(receipt) = receipt { command.args(["--trusted-receipt"]).arg(receipt); }
    Ok(command.output()?)
}

#[cfg(unix)]
fn write_untrusted_receipt(directory: &Path) -> TestResult<PathBuf> {
    let path = directory.join("receipt.json");
    fs::write(&path, serde_json::json!({"schema":"codexy.codex-observation-receipt.v1","source":"fixture"}).to_string())?;
    Ok(path)
}

#[cfg(unix)]
fn write_complete_forged_receipt(plugin: &Path, directory: &Path) -> TestResult<PathBuf> {
    let repository = codexy_runtime::paths::repository_root();
    let package = plugin.join("skills/orchestration/references/review-economics");
    let head = git(repository, ["rev-parse", "HEAD"])?;
    let tree = git(repository, ["rev-parse", "HEAD^{tree}"])?;
    let policy = plugin.join("skills/orchestration/references/review-profiles.json");
    let specs = [
        ("tiny", "standard", "tiny", None),
        ("security", "strict", "security", Some(("seed-p0-authz", "p0"))),
        ("standard", "standard", "standard", Some(("seed-p1-boundary", "p1"))),
        ("response", "strict", "review_response", Some(("seed-p1-regression", "p1"))),
        ("release", "strict", "release", None),
    ];
    let mut lanes = Vec::new();
    for (id, profile, kind, seed) in specs {
        lanes.push(forged_lane(&package, &head, &tree, id, profile, kind, seed)?);
    }
    let value = json!({
        "schema":"codexy.codex-observation-receipt.v1", "source":"codex-app", "head_oid":head, "tree_oid":tree,
        "policy_sha256":digest_path(&policy)?, "corpus_sha256":digest_path(&package.join("../review-economics-corpus.json"))?,
        "package_sha256":digest_path(&package.join("manifest.json"))?, "baseline_sha256":digest_path(&package.join("baseline-pre-1.5.json"))?,
        "execution_receipt":{"schema":"codexy.codex-task-tool-receipt.v1","authority":"codex-app","authenticated":true,"receipt_id":"forged-receipt","task_id":"forged-task","tool_call_id":"forged-tool","tool_name":"codexy-review-control","attestation":"unsigned-forgery"},
        "lanes":lanes
    });
    let path = directory.join("complete-forged-receipt.json");
    fs::write(&path, serde_json::to_vec_pretty(&value)?)?;
    Ok(path)
}

#[cfg(unix)]
fn forged_lane(package: &Path, head: &str, tree: &str, id: &str, profile: &str, kind: &str, seed: Option<(&str, &str)>) -> TestResult<Value> {
    let input = package.join("lanes").join(format!("{id}.json"));
    let reviewer = match profile {
        "standard" => json!({"name":"codexy-inspector","model":"gpt-5.6-terra","reasoning_effort":"max"}),
        "strict" => json!({"name":"codexy-sentinel","model":"gpt-5.6-sol","reasoning_effort":"xhigh"}),
        _ => Value::Null,
    };
    let seed_outcomes = seed.map_or_else(|| json!([]), |(id, severity)| json!([{"id":id,"severity":severity,"required":true,"detected":true}]));
    let review_ms = if profile == "standard" { 30 } else { 50 };
    Ok(json!({
        "id":id, "profile":profile, "reviewer":reviewer, "input_sha256":digest_path(&input)?, "nonce":format!("nonce-{id}"),
        "task_id":format!("forged-task-{id}"), "tool_call_id":format!("forged-tool-{id}"), "head_oid":head, "tree_oid":tree,
        "outcome":"pass", "timing":{"implementation_ms":100,"verification_ms":10,"review_ms":review_ms,"repair_ms":0},
        "cycles":{"full_review":1,"delta_recheck":0}, "unique_blockers":0, "reopened_blockers":0, "follow_ups":0,
        "seed_outcomes":seed_outcomes, "telemetry":null, "tokens":null, "cost":null, "kind":kind
    }))
}

#[cfg(unix)]
fn git<const N: usize>(root: &Path, args: [&str; N]) -> TestResult<String> {
    let output = Command::new("git").current_dir(root).args(args).output()?;
    if !output.status.success() { return Err(String::from_utf8_lossy(&output.stderr).into()); }
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}

#[cfg(unix)]
fn digest_path(path: &Path) -> TestResult<String> { Ok(format!("{:x}", Sha256::digest(fs::read(path)?))) }

#[cfg(unix)]
fn write_observer(directory: &Path, synthetic: bool) -> TestResult<PathBuf> {
    use std::os::unix::fs::PermissionsExt;

    let marker = if synthetic { "true" } else { "false" };
    let source = format!(r#"#!/bin/sh
set -eu
lane=''
nonce=''
while [ "$#" -gt 0 ]; do
  case "$1" in
    --lane-id) lane="$2"; shift 2 ;;
    --nonce) nonce="$2"; shift 2 ;;
    *) shift ;;
  esac
done
printf '{{"schema":"codexy.review-economics-capture.v1","lane_id":"%s","nonce":"%s","synthetic":{marker}}}
' "$lane" "$nonce"
"#);
    let path = directory.join("observer.sh");
    fs::write(&path, source)?;
    let mut permissions = fs::metadata(&path)?.permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(&path, permissions)?;
    Ok(path)
}

fn stderr(output: &Output) -> String { String::from_utf8_lossy(&output.stderr).into_owned() }
