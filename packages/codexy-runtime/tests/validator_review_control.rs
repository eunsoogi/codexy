use std::{fs, path::Path, process::Command};

use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};

use crate::support::TestResult;

#[test]
fn profiles_select_one_reviewer_with_fixed_models_and_escalation() -> TestResult {
    let fixture = crate::support::plugin_fixture()?;
    assert_profile(fixture.root(), "light", json!({"profile":"light","reviewer":null,"full_review_limit":0,"delta_recheck_limit":0}))?;
    assert_profile(fixture.root(), "standard", json!({"profile":"standard","reviewer":{"name":"codexy-inspector","model":"gpt-5.6-terra","reasoning_effort":"max"},"full_review_limit":1,"delta_recheck_limit":1}))?;
    assert_profile(fixture.root(), "strict", json!({"profile":"strict","reviewer":{"name":"codexy-sentinel","model":"gpt-5.6-sol","reasoning_effort":"xhigh"},"full_review_limit":1,"delta_recheck_limit":1,"discarded_lower_profile":"standard"}))
}

#[test]
fn packet_binds_real_git_state_and_durable_full_delta_budget() -> TestResult {
    let fixture = crate::support::plugin_fixture()?;
    let temp = tempfile::tempdir()?;
    let ledger = temp.path().join("ledger.json");
    let full = packet("e-full", "full");
    assert!(check_packet(fixture.root(), &ledger, &full)?.status.success());
    assert!(!check_packet(fixture.root(), &ledger, &full)?.status.success());
    let mut delta = packet("e-delta", "delta");
    delta["predecessor_event_id"] = json!("e-full");
    delta["budget"] = json!({"full_used":1,"delta_used":1});
    delta["findings"][0]["resolved"] = json!(true);
    delta["readiness_export"]["unresolved_blocker_ids"] = json!([]);
    delta["readiness_export"]["budget_exhausted"] = json!(true);
    assert!(check_packet(fixture.root(), &ledger, &delta)?.status.success());
    let mut parent = delta.clone();
    parent["event_id"] = json!("e-parent");
    parent["predecessor_event_id"] = json!("e-delta");
    parent["state"] = json!("parent_decision");
    parent["findings"][0]["reopen_count"] = json!(2);
    parent["readiness_export"]["parent_decision_required"] = json!(true);
    assert!(check_packet(fixture.root(), &ledger, &parent)?.status.success());
    let mut stale = packet("e-stale", "full");
    stale["identity"]["head_oid"] = json!(git(["rev-parse", "origin/main"]));
    assert!(!check_packet(fixture.root(), &temp.path().join("stale.json"), &stale)?.status.success());
    let mut wrong_diff = packet("e-wrong-diff", "full");
    wrong_diff["identity"]["diff_sha256"] = json!("0".repeat(64));
    assert!(!check_packet(fixture.root(), &temp.path().join("wrong-diff.json"), &wrong_diff)?.status.success());
    let mut invented = packet("e-invented", "full");
    invented["changed_files"] = json!(["missing.rs"]);
    assert!(!check_packet(fixture.root(), &temp.path().join("invented.json"), &invented)?.status.success());
    let mut omitted = packet("e-omitted", "full");
    omitted["readiness_export"]["unresolved_blocker_ids"] = json!([]);
    assert!(!check_packet(fixture.root(), &temp.path().join("omitted.json"), &omitted)?.status.success());
    Ok(())
}

#[test]
fn packet_stops_unavailable_or_recurrent_reviews_without_replacement() -> TestResult {
    let fixture = crate::support::plugin_fixture()?;
    let temp = tempfile::tempdir()?;
    let mut unavailable = packet("e-unavailable", "unobservable");
    unavailable["budget"] = json!({"full_used":0,"delta_used":0});
    unavailable["findings"] = json!([]);
    unavailable["resolution"] = json!({"repaired_finding_ids":[],"changed_boundaries":[]});
    unavailable["readiness_export"]["unresolved_blocker_ids"] = json!([]);
    unavailable["readiness_export"]["budget_exhausted"] = json!(false);
    unavailable["readiness_export"]["parent_decision_required"] = json!(true);
    assert!(check_packet(fixture.root(), &temp.path().join("unavailable.json"), &unavailable)?.status.success());
    unavailable["readiness_export"]["parent_decision_required"] = json!(false);
    assert!(!check_packet(fixture.root(), &temp.path().join("wrong-unavailable.json"), &unavailable)?.status.success());
    Ok(())
}

#[test]
fn packet_rejects_dual_unknown_inspector_overage_and_nonblocking_observations_pass() -> TestResult {
    let fixture = crate::support::plugin_fixture()?;
    let temp = tempfile::tempdir()?;
    let valid = packet("e-valid", "full");
    for mutate in [
        |value: &mut Value| value["reviewer"]["name"] = json!("codexy-sentinel"),
        |value: &mut Value| value["unknown"] = json!(true),
        too_many_blockers,
    ] {
        let mut invalid = valid.clone(); mutate(&mut invalid);
        assert!(!check_packet(fixture.root(), &temp.path().join(format!("{}.json", invalid["event_id"])), &invalid)?.status.success());
    }
    let mut strict = packet("e-strict", "full");
    strict["profile"] = json!("strict");
    strict["reviewer"] = json!({"name":"codexy-sentinel","model":"gpt-5.6-sol","reasoning_effort":"xhigh"});
    strict["readiness_export"]["profile"] = json!("strict");
    strict["readiness_export"]["reviewer"] = strict["reviewer"].clone();
    assert!(check_packet(fixture.root(), &temp.path().join("strict.json"), &strict)?.status.success());
    for kind in ["evidence_only", "github_metadata"] {
        let mut observed = packet(&format!("e-{kind}"), "full");
        observed["findings"][0]["kind"] = json!(kind);
        observed["readiness_export"]["unresolved_blocker_ids"] = json!([]);
        assert!(check_packet(fixture.root(), &temp.path().join(format!("{kind}.json")), &observed)?.status.success());
    }
    Ok(())
}

#[test]
fn named_inspector_precedes_generic_child_routing_without_caller_override() -> TestResult {
    let fixture = crate::support::plugin_fixture()?;
    let output = child_routing(fixture.root(), json!({"schema":"codexy.child-routing-request.v1","classification":"general","named_specialist":"codexy-inspector","codex_thread_operation":"create_thread"}))?;
    assert!(output.status.success());
    assert_eq!(serde_json::from_slice::<Value>(&output.stdout)?, json!({"route":"named_specialist","agent_type":"codexy-inspector"}));
    Ok(())
}

#[test]
fn economics_rejects_missing_parity_and_review_share_overages() -> TestResult {
    let fixture = crate::support::plugin_fixture()?; let mut valid = economics(); seed_outcomes(&mut valid);
    assert!(check_economics(fixture.root(), &valid)?.status.success());
    for mutate in [|value: &mut Value| value["lanes"][1]["baseline_p0"] = json!(2), |value: &mut Value| value["lanes"][0]["review_ms"] = json!(31), strict_overage] { let mut invalid = valid.clone(); mutate(&mut invalid); assert!(!check_economics(fixture.root(), &invalid)?.status.success()); }
    Ok(())
}

fn packet(event: &str, state: &str) -> Value {
    let base = git(["rev-parse", "origin/main"]); let head = git(["rev-parse", "HEAD"]);
    let diff = git_bytes(["diff", "--no-ext-diff", "--binary", &format!("{base}..{head}")]);
    let files = git(["diff", "--name-only", "--diff-filter=ACMR", &format!("{base}..{head}")]).lines().map(str::to_owned).collect::<Vec<_>>();
    let evidence_path = files.first().ok_or("current test head requires a changed file").unwrap();
    let evidence = git_bytes(["show", &format!("{head}:{evidence_path}")]);
    json!({"schema":"codexy.review-packet.v2","event_id":event,"predecessor_event_id":null,"profile":"standard","state":state,"reviewer":{"name":"codexy-inspector","model":"gpt-5.6-terra","reasoning_effort":"max"},"identity":{"base_oid":base,"head_oid":head,"diff_sha256":format!("{:x}",Sha256::digest(diff))},"acceptance_criteria":[{"id":"ac-1"}],"changed_files":files,"direct_boundaries":["validator"],"verification_results":[{"id":"evidence","head_oid":head,"evidence_path":evidence_path,"evidence_sha256":format!("{:x}",Sha256::digest(evidence))}],"findings":[{"id":"f-1","defect_class":"bounds","criterion_id":"ac-1","counterexample":"repro","head_oid":head,"kind":"blocker","reopen_count":0,"resolved":false}],"resolution":{"repaired_finding_ids":[],"changed_boundaries":[]},"budget":{"full_used":1,"delta_used":0},"readiness_export":{"head_oid":head,"profile":"standard","reviewer":{"name":"codexy-inspector","model":"gpt-5.6-terra","reasoning_effort":"max"},"unresolved_blocker_ids":["f-1"],"budget_exhausted":false,"parent_decision_required":false}})
}

fn assert_profile(root: &Path, profile: &str, expected: Value) -> TestResult { let request = if expected.get("discarded_lower_profile").is_some() { json!({"schema":"codexy.review-profile-request.v1","profile":profile,"prior_profile":"standard"}) } else { json!({"schema":"codexy.review-profile-request.v1","profile":profile}) }; let output = resolve_profile(root, request)?; assert!(output.status.success()); assert_eq!(serde_json::from_slice::<Value>(&output.stdout)?, expected); Ok(()) }
fn check_packet(root: &Path, ledger: &Path, value: &Value) -> TestResult<std::process::Output> { run(root, &["--repository-root", repository_root().to_str().ok_or("root")?, "--ledger", ledger.to_str().ok_or("ledger")?, "--check-packet"], value.clone()) }
fn resolve_profile(root: &Path, value: Value) -> TestResult<std::process::Output> { run(root, &["--resolve-profile"], value) }
fn check_economics(root: &Path, value: &Value) -> TestResult<std::process::Output> { run(root, &["--check-economics"], value.clone()) }
fn run(root: &Path, flags: &[&str], value: Value) -> TestResult<std::process::Output> { let temp = tempfile::tempdir()?; let input = temp.path().join("input.json"); fs::write(&input, serde_json::to_vec(&value)?)?; Ok(Command::new(env!("CARGO_BIN_EXE_codexy-review-control")).args(["--plugin-root", root.to_str().ok_or("plugin root")?]).args(flags).args(["--input", input.to_str().ok_or("input")?]).output()?) }
fn child_routing(root: &Path, value: Value) -> TestResult<std::process::Output> { let temp = tempfile::tempdir()?; let input = temp.path().join("request.json"); fs::write(&input, serde_json::to_vec(&value)?)?; Ok(Command::new(env!("CARGO_BIN_EXE_codexy-validate")).args(["--plugin-root", root.to_str().ok_or("plugin root")?, "--resolve-child-routing", "--routing-request-file"]).arg(input).output()?) }
fn git<const N: usize>(args: [&str; N]) -> String { String::from_utf8(git_bytes(args)).unwrap().trim().to_owned() }
fn git_bytes<const N: usize>(args: [&str; N]) -> Vec<u8> { Command::new("git").current_dir(repository_root()).args(args).output().unwrap().stdout }
fn repository_root() -> &'static Path { codexy_runtime::paths::repository_root() }
fn too_many_blockers(value: &mut Value) { for number in 2..=4 { let mut finding = value["findings"][0].clone(); finding["id"] = json!(format!("f-{number}")); value["findings"].as_array_mut().unwrap().push(finding); } }
fn strict_overage(value: &mut Value) { for lane in value["lanes"].as_array_mut().unwrap() { if lane["profile"] == "strict" { lane["review_ms"] = json!(51); } } }
fn seed_outcomes(value: &mut Value) { let lanes = value["lanes"].as_array_mut().unwrap(); let head = git(["rev-parse", "HEAD"]); for lane in &mut *lanes { lane["head_oid"] = json!(head); } lanes[0]["seed_outcomes"] = json!([]); lanes[1]["seed_outcomes"] = json!([{"id":"seed-p0-authz","detected":true}]); lanes[2]["seed_outcomes"] = json!([{"id":"seed-p1-boundary","detected":true}]); lanes[3]["seed_outcomes"] = json!([{"id":"seed-p1-regression","detected":true}]); lanes[4]["seed_outcomes"] = json!([]); }
fn economics() -> Value { json!({"schema":"codexy.review-economics.v1","lanes":[{"id":"tiny","kind":"tiny","profile":"standard","implementation_ms":100,"verification_ms":10,"review_ms":30,"repair_ms":0,"full_review_count":1,"delta_recheck_count":0,"unique_blockers":0,"reopened_blockers":0,"follow_ups":0,"baseline_p0":0,"observed_p0":0,"baseline_p1":0,"observed_p1":0,"tokens":null,"token_source":null,"review_share_ppm":214285},{"id":"security","kind":"security","profile":"strict","implementation_ms":100,"verification_ms":10,"review_ms":50,"repair_ms":0,"full_review_count":1,"delta_recheck_count":0,"unique_blockers":1,"reopened_blockers":0,"follow_ups":0,"baseline_p0":1,"observed_p0":1,"baseline_p1":0,"observed_p1":0,"tokens":10,"token_source":"runtime","review_share_ppm":312500},{"id":"standard","kind":"standard","profile":"standard","implementation_ms":100,"verification_ms":10,"review_ms":30,"repair_ms":0,"full_review_count":1,"delta_recheck_count":0,"unique_blockers":0,"reopened_blockers":0,"follow_ups":0,"baseline_p0":0,"observed_p0":0,"baseline_p1":1,"observed_p1":1,"tokens":null,"token_source":null,"review_share_ppm":214285},{"id":"response","kind":"review_response","profile":"strict","implementation_ms":100,"verification_ms":10,"review_ms":50,"repair_ms":0,"full_review_count":1,"delta_recheck_count":1,"unique_blockers":1,"reopened_blockers":1,"follow_ups":0,"baseline_p0":0,"observed_p0":0,"baseline_p1":1,"observed_p1":1,"tokens":null,"token_source":null,"review_share_ppm":312500},{"id":"release","kind":"release","profile":"strict","implementation_ms":100,"verification_ms":10,"review_ms":50,"repair_ms":0,"full_review_count":1,"delta_recheck_count":0,"unique_blockers":0,"reopened_blockers":0,"follow_ups":0,"baseline_p0":0,"observed_p0":0,"baseline_p1":0,"observed_p1":0,"tokens":null,"token_source":null,"review_share_ppm":312500}]}) }
