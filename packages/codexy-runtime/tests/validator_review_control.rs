use std::{fs, path::Path, process::Command};

use serde_json::{Value, json};

use crate::support::TestResult;

#[test]
fn review_profiles_route_one_named_reviewer_and_preserve_models() -> TestResult {
    let fixture = crate::support::plugin_fixture()?;
    assert_profile(fixture.root(), "light", json!({"profile":"light","reviewer":null,"full_review_limit":0,"delta_recheck_limit":0}))?;
    assert_profile(fixture.root(), "standard", json!({"profile":"standard","reviewer":{"name":"codexy-inspector","model":"gpt-5.6-terra","reasoning_effort":"max"},"full_review_limit":1,"delta_recheck_limit":1}))?;
    assert_profile(fixture.root(), "strict", json!({"profile":"strict","reviewer":{"name":"codexy-sentinel","model":"gpt-5.6-sol","reasoning_effort":"xhigh"},"full_review_limit":1,"delta_recheck_limit":1}))?;
    assert_profile(
        fixture.root(),
        "strict",
        json!({"profile":"strict","reviewer":{"name":"codexy-sentinel","model":"gpt-5.6-sol","reasoning_effort":"xhigh"},"full_review_limit":1,"delta_recheck_limit":1,"discarded_lower_profile":"standard"}),
    )?;
    Ok(())
}

#[test]
fn packet_rejects_dual_stale_duplicate_budget_and_reopen_violations() -> TestResult {
    let fixture = crate::support::plugin_fixture()?;
    let valid = packet();
    assert!(check_packet(fixture.root(), &valid)?.status.success());
    for mutate in [
        |value: &mut Value| value["reviewer"]["name"] = json!("codexy-sentinel"),
        |value: &mut Value| value["identity"]["head_sha"] = json!("stale"),
        duplicate_finding,
        |value: &mut Value| value["budget"]["full_used"] = json!(2),
        |value: &mut Value| value["findings"][0]["reopen_count"] = json!(2),
    ] {
        let mut invalid = valid.clone();
        mutate(&mut invalid);
        assert!(!check_packet(fixture.root(), &invalid)?.status.success());
    }
    Ok(())
}

#[test]
fn packet_limits_inspector_and_stops_the_second_recurrence_for_parent_decision() -> TestResult {
    let fixture = crate::support::plugin_fixture()?;
    let mut parent_decision = packet();
    parent_decision["state"] = json!("parent_decision");
    parent_decision["findings"][0]["reopen_count"] = json!(2);
    parent_decision["readiness_export"]["parent_decision_required"] = json!(true);
    assert!(check_packet(fixture.root(), &parent_decision)?.status.success());

    let mut too_many = packet();
    for number in 2..=4 {
        let mut finding = too_many["findings"][0].clone();
        finding["id"] = json!(format!("f-{number}"));
        too_many["findings"].as_array_mut().unwrap().push(finding);
    }
    assert!(!check_packet(fixture.root(), &too_many)?.status.success());

    let mut unknown = packet();
    unknown["unknown"] = json!(true);
    assert!(!check_packet(fixture.root(), &unknown)?.status.success());
    Ok(())
}

#[test]
fn packet_keeps_evidence_and_github_observations_nonblocking_without_replacement() -> TestResult {
    let fixture = crate::support::plugin_fixture()?;
    for kind in ["evidence_only", "github_metadata"] {
        let mut observed = packet();
        observed["findings"][0]["kind"] = json!(kind);
        observed["readiness_export"]["unresolved_blocker_ids"] = json!([]);
        assert!(check_packet(fixture.root(), &observed)?.status.success());
    }
    let mut unobservable = packet();
    unobservable["state"] = json!("unobservable");
    unobservable["budget"]["delta_used"] = json!(0);
    unobservable["findings"][0]["reopen_count"] = json!(0);
    unobservable["resolution"] = json!({"repaired_finding_ids":[],"changed_boundaries":[]});
    unobservable["readiness_export"]["budget_exhausted"] = json!(false);
    assert!(check_packet(fixture.root(), &unobservable)?.status.success());
    Ok(())
}

#[test]
fn named_inspector_precedes_generic_child_routing_without_caller_override() -> TestResult {
    let fixture = crate::support::plugin_fixture()?;
    let request = json!({
        "schema":"codexy.child-routing-request.v1",
        "classification":"general",
        "named_specialist":"codexy-inspector",
        "codex_thread_operation":"create_thread"
    });
    let output = child_routing(fixture.root(), request)?;
    assert!(output.status.success());
    assert_eq!(serde_json::from_slice::<Value>(&output.stdout)?, json!({"route":"named_specialist","agent_type":"codexy-inspector"}));
    Ok(())
}

#[test]
fn economics_rejects_missing_parity_and_review_share_overages() -> TestResult {
    let fixture = crate::support::plugin_fixture()?;
    let valid = economics();
    assert!(check_economics(fixture.root(), &valid)?.status.success());
    for mutate in [
        |value: &mut Value| value["lanes"][1]["baseline_p0"] = json!(2),
        |value: &mut Value| value["lanes"][0]["review_ms"] = json!(31),
        strict_overage,
    ] {
        let mut invalid = valid.clone();
        mutate(&mut invalid);
        assert!(!check_economics(fixture.root(), &invalid)?.status.success());
    }
    Ok(())
}

fn assert_profile(root: &Path, profile: &str, expected: Value) -> TestResult {
    let request = if expected.get("discarded_lower_profile").is_some() {
        json!({"schema":"codexy.review-profile-request.v1","profile":profile,"prior_profile":"standard"})
    } else { json!({"schema":"codexy.review-profile-request.v1","profile":profile}) };
    let output = resolve_profile(root, request)?;
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(serde_json::from_slice::<Value>(&output.stdout)?, expected);
    Ok(())
}

fn packet() -> Value {
    json!({
        "schema":"codexy.review-packet.v1","profile":"standard","state":"delta","reviewer":{"name":"codexy-inspector","model":"gpt-5.6-terra","reasoning_effort":"max"},
        "identity":{"base_sha":sha('a'),"head_sha":sha('b'),"diff_sha":sha('c')},
        "acceptance_criteria":[{"id":"ac-1"}],"changed_files":["src/lib.rs"],"direct_boundaries":["validator"],"verification_results":[{"id":"test","head_sha":sha('b'),"passed":true}],
        "findings":[{"id":"f-1","defect_class":"bounds","criterion_id":"ac-1","counterexample":"repro","head_sha":sha('b'),"kind":"blocker","reopen_count":1}],
        "resolution":{"repaired_finding_ids":["f-1"],"changed_boundaries":["validator"]},"budget":{"full_used":1,"delta_used":1},
        "readiness_export":{"head_sha":sha('b'),"profile":"standard","reviewer":{"name":"codexy-inspector","model":"gpt-5.6-terra","reasoning_effort":"max"},"unresolved_blocker_ids":["f-1"],"budget_exhausted":true,"parent_decision_required":false}
    })
}

fn economics() -> Value {
    json!({"schema":"codexy.review-economics.v1","lanes":[
        {"id":"tiny","kind":"tiny","profile":"standard","implementation_ms":100,"verification_ms":10,"review_ms":30,"repair_ms":0,"full_review_count":1,"delta_recheck_count":0,"unique_blockers":0,"reopened_blockers":0,"follow_ups":0,"baseline_p0":0,"observed_p0":0,"baseline_p1":0,"observed_p1":0,"tokens":null,"token_source":null,"review_share_ppm":214285},
        {"id":"security","kind":"security","profile":"strict","implementation_ms":100,"verification_ms":10,"review_ms":50,"repair_ms":0,"full_review_count":1,"delta_recheck_count":0,"unique_blockers":1,"reopened_blockers":0,"follow_ups":0,"baseline_p0":1,"observed_p0":1,"baseline_p1":0,"observed_p1":0,"tokens":10,"token_source":"runtime","review_share_ppm":312500},
        {"id":"standard","kind":"standard","profile":"standard","implementation_ms":100,"verification_ms":10,"review_ms":30,"repair_ms":0,"full_review_count":1,"delta_recheck_count":0,"unique_blockers":0,"reopened_blockers":0,"follow_ups":0,"baseline_p0":0,"observed_p0":0,"baseline_p1":1,"observed_p1":1,"tokens":null,"token_source":null,"review_share_ppm":214285},
        {"id":"response","kind":"review_response","profile":"strict","implementation_ms":100,"verification_ms":10,"review_ms":50,"repair_ms":0,"full_review_count":1,"delta_recheck_count":1,"unique_blockers":1,"reopened_blockers":1,"follow_ups":0,"baseline_p0":0,"observed_p0":0,"baseline_p1":1,"observed_p1":1,"tokens":null,"token_source":null,"review_share_ppm":312500},
        {"id":"release","kind":"release","profile":"strict","implementation_ms":100,"verification_ms":10,"review_ms":50,"repair_ms":0,"full_review_count":1,"delta_recheck_count":0,"unique_blockers":0,"reopened_blockers":0,"follow_ups":0,"baseline_p0":0,"observed_p0":0,"baseline_p1":0,"observed_p1":0,"tokens":null,"token_source":null,"review_share_ppm":312500}
    ]})
}

fn sha(ch: char) -> String { std::iter::repeat_n(ch, 64).collect() }

fn duplicate_finding(value: &mut Value) {
    let finding = value["findings"][0].clone();
    value["findings"].as_array_mut().unwrap().push(finding);
}

fn strict_overage(value: &mut Value) {
    for lane in value["lanes"].as_array_mut().unwrap() {
        if lane["profile"] == "strict" { lane["review_ms"] = json!(51); }
    }
}

fn resolve_profile(root: &Path, value: Value) -> TestResult<std::process::Output> { run(root, "--resolve-profile", value) }
fn check_packet(root: &Path, value: &Value) -> TestResult<std::process::Output> { run(root, "--check-packet", value.clone()) }
fn check_economics(root: &Path, value: &Value) -> TestResult<std::process::Output> { run(root, "--check-economics", value.clone()) }

fn run(root: &Path, mode: &str, value: Value) -> TestResult<std::process::Output> {
    let temp = tempfile::tempdir()?;
    let input = temp.path().join("input.json");
    fs::write(&input, serde_json::to_vec(&value)?)?;
    Ok(Command::new(env!("CARGO_BIN_EXE_codexy-review-control"))
        .args(["--plugin-root", root.to_str().ok_or("plugin root")?, mode, "--input"])
        .arg(input)
        .output()?)
}

fn child_routing(root: &Path, value: Value) -> TestResult<std::process::Output> {
    let temp = tempfile::tempdir()?;
    let input = temp.path().join("request.json");
    fs::write(&input, serde_json::to_vec(&value)?)?;
    Ok(Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args(["--plugin-root", root.to_str().ok_or("plugin root")?, "--resolve-child-routing", "--routing-request-file"])
        .arg(input)
        .output()?)
}
