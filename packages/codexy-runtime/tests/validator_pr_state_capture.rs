use std::{fs, process::Command};

use serde_json::{Value, json};

use crate::support::TestResult;

#[test]
fn canonical_capture_preserves_typed_standard_and_light_contracts() -> TestResult {
    let standard = capture(standard_contract())?;
    assert_eq!(standard["reviewProfile"], "standard");
    assert_eq!(standard["reviewEvidence"]["event_id"], "e-passed");
    assert_eq!(standard["reviewLedger"]["events"][1]["id"], "e-passed");

    let light = capture(json!({
        "reviewProfile":"light",
        "reviewDecision":"NOT_REQUIRED"
    }))?;
    assert_eq!(light["reviewProfile"], "light");
    assert!(light.get("reviewEvidence").is_none());
    assert!(light.get("reviewLedger").is_none());
    Ok(())
}

#[test]
fn canonical_capture_rejects_invalid_typed_review_contracts() -> TestResult {
    for review in [
        json!({"reviewProfile":"standard","reviewDecision":"APPROVED"}),
        json!({
            "reviewProfile":"light",
            "reviewDecision":"NOT_REQUIRED",
            "reviewEvidence":{}
        }),
        json!({
            "reviewProfile":"standard", "reviewDecision":"APPROVED",
            "reviewEvidence":{"profile":"strict","head_oid":"head"}, "reviewLedger":{}
        }),
        json!({
            "reviewProfile":"standard", "reviewDecision":"APPROVED",
            "reviewEvidence":{"profile":"standard","head_oid":"stale"}, "reviewLedger":{}
        }),
        json!({"reviewProfile":"unknown","reviewDecision":"APPROVED"}),
    ] {
        assert!(!capture_output(review)?.status.success());
    }
    Ok(())
}

fn capture(review: Value) -> TestResult<Value> {
    let temp = tempfile::tempdir()?;
    let base = temp.path().join("base.json");
    let control = temp.path().join("review-control.json");
    let state = temp.path().join("pr-state.json");
    fs::write(&base, serde_json::to_vec(&json!({"number":562,"headRefOid":"head"}))?)?;
    fs::write(&control, serde_json::to_vec(&review)?)?;
    let output = run_capture(&base, &control, &state)?;
    assert!(
        output.status.success(),
        "capture must preserve a valid typed review contract: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(serde_json::from_slice(&fs::read(state)?)?)
}

fn capture_output(review: Value) -> TestResult<std::process::Output> {
    let temp = tempfile::tempdir()?;
    let base = temp.path().join("base.json");
    let control = temp.path().join("review-control.json");
    let output = temp.path().join("pr-state.json");
    fs::write(&base, serde_json::to_vec(&json!({"number":562,"headRefOid":"head"}))?)?;
    fs::write(&control, serde_json::to_vec(&review)?)?;
    run_capture(&base, &control, &output)
}

fn run_capture(
    base: &std::path::Path,
    control: &std::path::Path,
    output: &std::path::Path,
) -> TestResult<std::process::Output> {
    Ok(Command::new(codexy_runtime::paths::repository_root().join("scripts/build-pr-state"))
        .args([
            "--base-pr-state-file",
            base.to_str().ok_or("base path")?,
            "--review-control-state-file",
            control.to_str().ok_or("review path")?,
            "--output",
            output.to_str().ok_or("output path")?,
        ])
        .output()?)
}

fn standard_contract() -> Value {
    json!({
        "reviewProfile":"standard", "reviewDecision":"APPROVED",
        "reviewEvidence":{"schema":"codexy.review-readiness.v1","head_oid":"head","profile":"standard","reviewer":{"name":"codexy-inspector","model":"gpt-5.6-terra","reasoning_effort":"max"},"state":"passed","event_id":"e-passed","blockers":[]},
        "reviewLedger":{"schema":"codexy.review-ledger.v1","events":[
            {"id":"e-full","predecessor_event_id":null,"profile":"standard","base_oid":"base","head_oid":"head","state":"full","full_used":1,"delta_used":0,"blockers":[],"boundaries":["validator"],"escalation":null},
            {"id":"e-passed","predecessor_event_id":"e-full","profile":"standard","base_oid":"base","head_oid":"head","state":"passed","full_used":1,"delta_used":0,"blockers":[],"boundaries":["validator"],"escalation":null}
        ]}
    })
}
