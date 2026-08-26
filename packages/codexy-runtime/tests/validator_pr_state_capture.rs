use std::{fs, path::Path, process::Command};

use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};

use crate::support::{FixtureCommand, TestResult};

#[test]
fn review_control_producer_captures_a_bound_terminal_without_appending_an_event() -> TestResult {
    let fixture = crate::support::plugin_fixture()?;
    let temp = tempfile::tempdir()?;
    let input = temp.path().join("producer-request.json");
    fs::write(&input, serde_json::to_vec(&producer_request()?)?)?;
    let output = run_producer(fixture.root(), &input, temp.path())?;
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let control: Value = serde_json::from_slice(&fs::read(temp.path().join("control.json"))?)?;
    let packet: Value = serde_json::from_slice(&fs::read(temp.path().join("packet.json"))?)?;
    let ledger: Value = serde_json::from_slice(&fs::read(temp.path().join("ledger.json"))?)?;
    assert_eq!(control["schema"], "codexy.review-control-state.v1");
    assert_eq!(packet["schema"], "codexy.review-packet.v4");
    assert_eq!(ledger["events"].as_array().map(Vec::len), Some(2));
    Ok(())
}

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
        json!({"schema":"codexy.review-control-state.v1","profile":"standard","decision":"APPROVED","evidence":null,"ledger":null}),
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

#[test]
fn canonical_capture_requires_nullable_field_presence() -> TestResult {
    for mutate in [
        |control: &mut Value| {
            control["ledger"]["events"][0]
                .as_object_mut()
                .expect("full event")
                .remove("predecessor_event_id");
        },
        |control: &mut Value| {
            control["ledger"]["events"][1]
                .as_object_mut()
                .expect("terminal event")
                .remove("predecessor_event_id");
        },
        |control: &mut Value| {
            control["ledger"]["events"][0]
                .as_object_mut()
                .expect("full event")
                .remove("escalation");
        },
        |control: &mut Value| {
            control["ledger"]["events"][1]
                .as_object_mut()
                .expect("terminal event")
                .remove("escalation");
        },
    ] {
        let mut control = standard_control();
        mutate(&mut control);
        assert!(
            !capture_output(control)?.status.success(),
            "every required nullable ledger field must be present"
        );
    }
    for field in ["evidence", "ledger"] {
        let mut light = json!({
            "schema":"codexy.review-control-state.v1",
            "profile":"light",
            "decision":"NOT_REQUIRED"
        });
        light[field] = Value::Null;
        assert!(
            !capture_output(light)?.status.success(),
            "light review control must omit {field}, not attach null"
        );
    }
    Ok(())
}

#[test]
fn canonical_capture_accepts_explicit_nullable_ledger_values() -> TestResult {
    for control in [standard_control(), strict_control()] {
        let state = capture(control)?;
        let events = state["reviewControl"]["ledger"]["events"]
            .as_array()
            .expect("events");
        assert!(events.iter().all(|event| event["escalation"].is_null()));
        assert!(events[0]["predecessor_event_id"].is_null());
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

fn producer_request() -> TestResult<Value> {
    // Use HEAD^ so the fixture remains non-empty after a PR merges into main.
    let base = git(&["rev-parse", "HEAD^"]);
    let head = git(&["rev-parse", "HEAD"]);
    let range = format!("{base}..{head}");
    let diff = git_bytes(&["diff", "--no-ext-diff", "--binary", &range]);
    let files = git(&["diff", "--name-only", "--diff-filter=ACMRD", &range])
        .lines()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let evidence_path = files.first().ok_or("producer needs changed-file evidence")?;
    let evidence = git_bytes(&["show", &format!("{head}:{evidence_path}")]);
    let diff_sha = format!("{:x}", Sha256::digest(diff));
    let evidence_sha = format!("{:x}", Sha256::digest(evidence));
    let mut ledger = strict_control()["ledger"].clone();
    for event in ledger["events"].as_array_mut().ok_or("ledger events")? {
        event["base_oid"] = json!(base);
        event["head_oid"] = json!(head);
    }
    let contract = ledger["events"][0]["issue_contract"].clone();
    Ok(json!({"schema":"codexy.review-control-producer-request.v1","binding":{"issue_number":693,"pull_request_number":694,"base_oid":base,"head_oid":head,"diff_sha256":diff_sha,"profile":"strict","reviewer":{"name":"codexy-sentinel","model":"gpt-5.6-sol","reasoning_effort":"xhigh"},"event_id":"e-passed","predecessor_event_id":"e-full","issue_contract":contract,"budget":{"full_used":1,"delta_used":0,"terminal_used":2,"terminal_limit":3}},"terminal_record":{"schema":"codexy.review-terminal-record.v1","head_oid":head,"profile":"strict","reviewer":{"name":"codexy-sentinel","model":"gpt-5.6-sol","reasoning_effort":"xhigh"},"state":"passed","event_id":"e-passed","blockers":[],"ledger":ledger},"packet":{"schema":"codexy.review-packet.v4","event_id":"e-passed","predecessor_event_id":"e-full","profile":"strict","state":"passed","reviewer":{"name":"codexy-sentinel","model":"gpt-5.6-sol","reasoning_effort":"xhigh"},"identity":{"base_oid":base,"head_oid":head,"diff_sha256":diff_sha},"issue_contract":contract,"changed_files":files,"direct_boundaries":["validator"],"verification_results":[{"id":"evidence","head_oid":head,"evidence_path":evidence_path,"evidence_sha256":evidence_sha}],"findings":[],"resolution":{"repaired_finding_ids":[],"changed_boundaries":[]},"budget":{"full_used":1,"delta_used":0},"readiness_export":{"head_oid":head,"profile":"strict","reviewer":{"name":"codexy-sentinel","model":"gpt-5.6-sol","reasoning_effort":"xhigh"},"unresolved_blocker_ids":[],"budget_exhausted":false,"parent_decision_required":false}}}))
}

fn run_producer(root: &Path, input: &Path, output_dir: &Path) -> TestResult<std::process::Output> {
    Ok(Command::new(env!("CARGO_BIN_EXE_codexy-review-control"))
        .args(["--plugin-root", root.to_str().ok_or("plugin root")?, "--produce-review-control", "--input"])
        .arg(input)
        .args(["--output"])
        .arg(output_dir.join("control.json"))
        .args(["--packet-output"])
        .arg(output_dir.join("packet.json"))
        .args(["--ledger-output"])
        .arg(output_dir.join("ledger.json"))
        .output()?)
}

fn git(args: &[&str]) -> String {
    String::from_utf8(Command::new("git").current_dir(codexy_runtime::paths::repository_root()).args(args).output().unwrap().stdout).unwrap().trim().to_owned()
}

fn git_bytes(args: &[&str]) -> Vec<u8> {
    Command::new("git").current_dir(codexy_runtime::paths::repository_root()).args(args).output().unwrap().stdout
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
    let mut command = FixtureCommand::new(
        codexy_runtime::paths::repository_root().join("scripts/build-pr-state"),
    );
    command
        .arg("--base-pr-state-file")
        .arg_path(base)
        .arg("--review-control-state-file")
        .arg_path(control)
        .arg("--output")
        .arg_path(output)
        .env_path(
            "CODEXY_REVIEW_CONTROL_BIN",
            env!("CARGO_BIN_EXE_codexy-review-control"),
        );
    Ok(command.output()?)
}

fn standard_control() -> Value { control("standard", "APPROVED", "passed", "codexy-inspector", "gpt-5.6-terra", "max") }
fn strict_control() -> Value { control("strict", "APPROVED", "passed", "codexy-sentinel", "gpt-5.6-sol", "xhigh") }

fn control(profile: &str, decision: &str, state: &str, name: &str, model: &str, reasoning_effort: &str) -> Value {
    json!({
        "schema":"codexy.review-control-state.v1", "profile":profile, "decision":decision,
        "evidence":{"schema":"codexy.review-readiness.v1","head_oid":"head","profile":profile,"reviewer":{"name":name,"model":model,"reasoning_effort":reasoning_effort},"state":state,"event_id":"e-passed","blockers":[]},
        "ledger":{"schema":"codexy.review-ledger.v1","events":[
            {"id":"e-full","predecessor_event_id":null,"profile":profile,"base_oid":"base","head_oid":"head","state":"full","full_used":1,"delta_used":0,"blockers":[],"boundaries":["validator"],"issue_contract":{"problem":"owned problem","scope":"owned scope","acceptance_criteria":[{"id":"ac-1"}],"owned_invariant_ids":[],"exclusions":[],"adjacent_dependencies":[]},"issue_contract_sha256":"30e2a0c55aa2db0a84e6924f5a4731f335ea652f79123af992903d8ec1c617e2","escalation":null},
            {"id":"e-passed","predecessor_event_id":"e-full","profile":profile,"base_oid":"base","head_oid":"head","state":state,"full_used":1,"delta_used":0,"blockers":[],"boundaries":["validator"],"issue_contract":{"problem":"owned problem","scope":"owned scope","acceptance_criteria":[{"id":"ac-1"}],"owned_invariant_ids":[],"exclusions":[],"adjacent_dependencies":[]},"issue_contract_sha256":"30e2a0c55aa2db0a84e6924f5a4731f335ea652f79123af992903d8ec1c617e2","escalation":null}
        ]}
    })
}
