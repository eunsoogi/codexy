use std::{fs, process::Command};

use serde_json::{Value, json};

use crate::support::TestResult;

#[test]
fn canonical_capture_preserves_github_and_typed_terminal_decisions() -> TestResult {
    let standard = capture(standard_control())?;
    assert_eq!(standard["reviewDecision"], "CHANGES_REQUESTED");
    assert_eq!(standard["reviewControl"]["profile"], "standard");
    assert_eq!(standard["reviewControl"]["decision"], "APPROVED");
    assert_eq!(standard["reviewControl"]["evidence"]["event_id"], "e-passed");

    let strict = capture(strict_control())?;
    assert_eq!(strict["reviewControl"]["profile"], "strict");
    assert_eq!(strict["reviewControl"]["evidence"]["reviewer"]["name"], "codexy-sentinel");

    let light = capture(json!({
        "schema":"codexy.review-control-state.v1",
        "profile":"light",
        "decision":"NOT_REQUIRED"
    }))?;
    assert_eq!(light["reviewDecision"], "CHANGES_REQUESTED");
    assert_eq!(light["reviewControl"]["profile"], "light");
    assert!(light["reviewControl"].get("evidence").is_none());
    assert!(light["reviewControl"].get("ledger").is_none());
    Ok(())
}

#[test]
fn canonical_capture_rejects_invalid_typed_terminal_contracts() -> TestResult {
    for review in [
        json!({"schema":"codexy.review-control-state.v1","profile":"standard","decision":"APPROVED"}),
        json!({"schema":"codexy.review-control-state.v1","profile":"light","decision":"NOT_REQUIRED","evidence":{}}),
        json!({"schema":"codexy.review-control-state.v1","profile":"standard","decision":"INVALID","evidence":{},"ledger":{}}),
        json!({"schema":"codexy.review-control-state.v1","profile":"standard","decision":"APPROVED","evidence":{"profile":"strict","head_oid":"head"},"ledger":{}}),
        json!({"schema":"codexy.review-control-state.v1","profile":"standard","decision":"APPROVED","evidence":{"profile":"standard","head_oid":"stale"},"ledger":{}}),
        json!({"schema":"codexy.review-control-state.v1","profile":"unknown","decision":"APPROVED"}),
    ] {
        assert!(!capture_output(review)?.status.success());
    }
    Ok(())
}

#[test]
fn canonical_capture_rejects_invalid_nonterminal_ledger_history() -> TestResult {
    for mutate in [
        |control: &mut Value| control["ledger"]["events"][0]["head_oid"] = json!("stale"),
        |control: &mut Value| control["ledger"]["events"][0]["base_oid"] = json!("stale"),
        |control: &mut Value| control["ledger"]["events"][1]["predecessor_event_id"] = json!("other"),
        |control: &mut Value| control["ledger"]["events"].as_array_mut().expect("events").reverse(),
        |control: &mut Value| control["ledger"]["events"][0]["full_used"] = json!(0),
        |control: &mut Value| control["ledger"]["events"][1]["delta_used"] = json!(1),
        |control: &mut Value| control["ledger"]["events"][0]["full_used"] = json!("one"),
        |control: &mut Value| control["ledger"]["events"][0]["boundaries"] = json!([]),
        |control: &mut Value| control["ledger"]["events"][0]["blockers"] = json!({}),
        |control: &mut Value| control["ledger"]["events"][0]["id"] = json!("e-passed"),
        |control: &mut Value| control["ledger"]["events"][0]["extra"] = json!(true),
    ] {
        let mut control = standard_control();
        mutate(&mut control);
        assert!(
            !capture_output(control)?.status.success(),
            "capture must reject malformed nonterminal review history"
        );
    }
    Ok(())
}

fn capture(review: Value) -> TestResult<Value> {
    let temp = tempfile::tempdir()?;
    let (base, control, state) = state_files(temp.path(), &review)?;
    let output = run_capture(&base, &control, &state)?;
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    Ok(serde_json::from_slice(&fs::read(state)?)?)
}

fn capture_output(review: Value) -> TestResult<std::process::Output> {
    let temp = tempfile::tempdir()?;
    let (base, control, output) = state_files(temp.path(), &review)?;
    run_capture(&base, &control, &output)
}

fn state_files(root: &std::path::Path, review: &Value) -> TestResult<(std::path::PathBuf, std::path::PathBuf, std::path::PathBuf)> {
    let base = root.join("base.json");
    let control = root.join("review-control.json");
    let output = root.join("pr-state.json");
    fs::write(&base, serde_json::to_vec(&json!({"number":562,"headRefOid":"head","reviewDecision":"CHANGES_REQUESTED"}))?)?;
    fs::write(&control, serde_json::to_vec(review)?)?;
    Ok((base, control, output))
}

fn run_capture(base: &std::path::Path, control: &std::path::Path, output: &std::path::Path) -> TestResult<std::process::Output> {
    Ok(Command::new(codexy_runtime::paths::repository_root().join("scripts/build-pr-state"))
        .args(["--base-pr-state-file", base.to_str().ok_or("base path")?, "--review-control-state-file", control.to_str().ok_or("review path")?, "--output", output.to_str().ok_or("output path")?])
        .env("CODEXY_REVIEW_CONTROL_BIN", env!("CARGO_BIN_EXE_codexy-review-control"))
        .output()?)
}

fn standard_control() -> Value { control("standard", "APPROVED", "passed", "codexy-inspector", "gpt-5.6-terra", "max") }
fn strict_control() -> Value { control("strict", "APPROVED", "passed", "codexy-sentinel", "gpt-5.6-sol", "xhigh") }

fn control(profile: &str, decision: &str, state: &str, name: &str, model: &str, reasoning_effort: &str) -> Value {
    json!({
        "schema":"codexy.review-control-state.v1", "profile":profile, "decision":decision,
        "evidence":{"schema":"codexy.review-readiness.v1","head_oid":"head","profile":profile,"reviewer":{"name":name,"model":model,"reasoning_effort":reasoning_effort},"state":state,"event_id":"e-passed","blockers":[]},
        "ledger":{"schema":"codexy.review-ledger.v1","events":[
            {"id":"e-full","predecessor_event_id":null,"profile":profile,"base_oid":"base","head_oid":"head","state":"full","full_used":1,"delta_used":0,"blockers":[],"boundaries":["validator"],"escalation":null},
            {"id":"e-passed","predecessor_event_id":"e-full","profile":profile,"base_oid":"base","head_oid":"head","state":state,"full_used":1,"delta_used":0,"blockers":[],"boundaries":["validator"],"escalation":null}
        ]}
    })
}
