use std::{fs, path::Path};

use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};

use crate::support::TestResult;

const PACKAGE: &str = "skills/orchestration/references/review-economics";
const MANIFEST: &str = "skills/orchestration/references/review-economics/manifest.json";
const TINY_INPUT: &str = "skills/orchestration/references/review-economics/lanes/tiny.json";
const STRICT_TRIGGERS: [&str; 11] = [
    "destructive",
    "security",
    "permission",
    "secret",
    "release",
    "high_consequence_external_state",
    "high_risk_guardrail",
    "merge_sensitive",
    "durable_delegation",
    "multi_lane_ownership",
    "explicit_audit_evidence",
];

#[test]
fn tiny_lane_selects_authoritative_light_without_a_reviewer() -> TestResult {
    let fixture = crate::support::plugin_fixture()?;
    let tiny: Value = serde_json::from_slice(&fs::read(fixture.root().join(TINY_INPUT))?)?;
    assert_eq!(tiny["profile"], "light");
    let request = json!({
        "schema":"codexy.review-profile-request.v1",
        "classification": {
            "schema":"codexy.workflow-profile-classification.v2",
            "work_class":"low_risk",
            "low_risk_eligible":true,
            "strict_triggers":STRICT_TRIGGERS.iter().map(|kind| json!({"kind":kind,"applies":false})).collect::<Vec<_>>()
        }
    });
    let output = super::resolve_profile(fixture.root(), request)?;
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout)?,
        json!({"profile":"light","reviewer":null,"full_review_limit":0,"delta_recheck_limit":0})
    );
    Ok(())
}

#[test]
fn package_accepts_a_nested_lane_input_path() -> TestResult {
    let stderr = package_check(Some("lanes/nested/tiny.json"), None, true)?;
    assert!(stderr.contains("no callable verifier"), "{stderr}");
    Ok(())
}

#[test]
fn package_rejects_cross_platform_unsafe_lane_paths() -> TestResult {
    for path in ["../outside.json", r"..\outside.json", "/outside.json", r"C:\outside.json"] {
        let stderr = package_check(Some(path), None, false)?;
        assert!(stderr.contains("unsafe"), "{path}: {stderr}");
    }
    Ok(())
}

#[test]
fn package_rejects_malformed_and_cross_document_lane_paths() -> TestResult {
    let malformed = package_check(None, None, false)?;
    assert!(malformed.contains("missing string field: path"), "{malformed}");
    let mismatch = package_check(Some("lanes/tiny.json"), Some("standard"), false)?;
    assert!(mismatch.contains("bound to its manifest and corpus"), "{mismatch}");
    Ok(())
}

fn package_check(
    path: Option<&str>,
    tiny_profile: Option<&str>,
    nested: bool,
) -> TestResult<String> {
    let mutable = [Path::new(MANIFEST), Path::new(TINY_INPUT)];
    let fixture = crate::support::plugin_fixture_with_mutable_files(&mutable)?;
    let manifest_path = fixture.root().join(MANIFEST);
    let mut manifest: Value = serde_json::from_slice(&fs::read(&manifest_path)?)?;
    let lane = manifest["lanes"]
        .as_array_mut()
        .and_then(|lanes| lanes.iter_mut().find(|lane| lane["id"] == "tiny"))
        .ok_or("tiny manifest lane")?;
    lane["path"] = path.map_or(Value::Null, |path| json!(path));
    fs::write(&manifest_path, serde_json::to_vec(&manifest)?)?;
    let tiny_path = fixture.root().join(TINY_INPUT);
    if let Some(profile) = tiny_profile {
        let mut tiny: Value = serde_json::from_slice(&fs::read(&tiny_path)?)?;
        tiny["profile"] = json!(profile);
        fs::write(&tiny_path, serde_json::to_vec(&tiny)?)?;
    }
    if nested {
        let nested_path = fixture.root().join(PACKAGE).join("lanes/nested/tiny.json");
        fs::create_dir_all(nested_path.parent().ok_or("nested lane parent")?)?;
        fs::copy(&tiny_path, nested_path)?;
    }
    let output = super::check_economics(fixture.root(), &unavailable())?;
    assert!(!output.status.success(), "package unexpectedly accepted");
    Ok(String::from_utf8_lossy(&output.stderr).into_owned())
}

pub(super) fn report() -> Value {
    json!({"schema":"codexy.review-economics.v2","status":"observed","head_oid":"","policy_sha256":"","corpus_sha256":"","lanes":[
        lane("tiny", "tiny", "standard", 30, 1, 0, 0, 0, 0, 0, 0, None),
        lane("security", "security", "strict", 50, 1, 0, 1, 0, 1, 1, 0, Some(10)),
        lane("standard", "standard", "standard", 30, 1, 0, 0, 0, 0, 0, 1, None),
        lane("response", "review_response", "strict", 50, 1, 1, 1, 1, 0, 0, 1, None),
        lane("release", "release", "strict", 50, 1, 0, 0, 0, 0, 0, 0, None)
    ]})
}

pub(super) fn bind(value: &mut Value, root: &Path, head: &str) {
    value["head_oid"] = json!(head);
    value["policy_sha256"] = json!(digest(root.join("skills/orchestration/references/review-profiles.json")));
    value["corpus_sha256"] = json!(digest(root.join("skills/orchestration/references/review-economics-corpus.json")));
    let lanes = value["lanes"].as_array_mut().unwrap();
    for lane in &mut *lanes { lane["head_oid"] = json!(head); }
    lanes[0]["seed_outcomes"] = json!([]);
    lanes[1]["seed_outcomes"] = json!([{"id":"seed-p0-authz","detected":true}]);
    lanes[2]["seed_outcomes"] = json!([{"id":"seed-p1-boundary","detected":true}]);
    lanes[3]["seed_outcomes"] = json!([{"id":"seed-p1-regression","detected":true}]);
    lanes[4]["seed_outcomes"] = json!([]);
}

pub(super) fn unavailable() -> Value {
    json!({"schema":"codexy.review-economics.v2","status":"unavailable","head_oid":null,"policy_sha256":null,"corpus_sha256":null,"lanes":[],"reason":"repository-owned observation has not been captured"})
}

pub(super) fn strict_overage(value: &mut Value) {
    for lane in value["lanes"].as_array_mut().unwrap() {
        if lane["profile"] == "strict" { lane["review_ms"] = json!(51); }
    }
}

fn lane(id: &str, kind: &str, profile: &str, review_ms: u64, full: u8, delta: u8, blockers: u32, reopened: u32, p0: u32, observed_p0: u32, p1: u32, tokens: Option<u64>) -> Value {
    let token_source = tokens.map(|_| "runtime");
    json!({"id":id,"kind":kind,"profile":profile,"head_oid":"","implementation_ms":100,"verification_ms":10,"review_ms":review_ms,"repair_ms":0,"full_review_count":full,"delta_recheck_count":delta,"unique_blockers":blockers,"reopened_blockers":reopened,"follow_ups":0,"baseline_p0":p0,"observed_p0":observed_p0,"baseline_p1":p1,"observed_p1":p1,"tokens":tokens,"token_source":token_source,"seed_outcomes":[]})
}

fn digest(path: impl AsRef<Path>) -> String {
    format!("{:x}", Sha256::digest(fs::read(path).unwrap()))
}
