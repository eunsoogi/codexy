use std::{fs, path::Path, process::Command, sync::OnceLock};

use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};

use crate::support::TestResult;

#[path = "validator_review_control/economics.rs"]
mod review_economics;
#[path = "validator_review_control/ledger.rs"]
mod review_ledger;
#[path = "validator_review_control/escalation.rs"]
mod review_escalation;
#[path = "validator_review_control/terminal_scope.rs"]
mod terminal_scope;
#[path = "validator_review_control/handoff_decision.rs"]
mod handoff_decision;
#[path = "validator_review_control/profile_classification.rs"]
mod profile_classification;
#[path = "validator_review_control/issue_contract.rs"]
mod issue_contract;

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
    let mut unresolved_pass = packet("e-unresolved-pass", "passed");
    unresolved_pass["predecessor_event_id"] = json!("e-full");
    assert!(!check_packet(fixture.root(), &ledger, &unresolved_pass)?.status.success());
    let mut stale = packet("e-stale", "full");
    stale["identity"]["head_oid"] =
        json!(git_at(packet_repository(), ["rev-parse", "HEAD^"])?);
    assert!(!check_packet(fixture.root(), &temp.path().join("stale.json"), &stale)?.status.success());
    let mut wrong_diff = packet("e-wrong-diff", "full");
    wrong_diff["identity"]["diff_sha256"] = json!("0".repeat(64));
    assert!(!check_packet(fixture.root(), &temp.path().join("wrong-diff.json"), &wrong_diff)?.status.success());
    let mut symbolic_base = packet("e-symbolic-base", "full");
    symbolic_base["identity"]["base_oid"] = json!("origin/main");
    assert!(!check_packet(fixture.root(), &temp.path().join("symbolic-base.json"), &symbolic_base)?.status.success());
    let mut duplicate_files = packet("e-duplicate-files", "full");
    let file = duplicate_files["changed_files"][0].clone();
    duplicate_files["changed_files"].as_array_mut().unwrap().push(file);
    assert!(!check_packet(fixture.root(), &temp.path().join("duplicate-files.json"), &duplicate_files)?.status.success());
    let mut invented = packet("e-invented", "full");
    invented["changed_files"] = json!(["missing.rs"]);
    assert!(!check_packet(fixture.root(), &temp.path().join("invented.json"), &invented)?.status.success());
    let mut omitted = packet("e-omitted", "full");
    omitted["readiness_export"]["unresolved_blocker_ids"] = json!([]);
    assert!(!check_packet(fixture.root(), &temp.path().join("omitted.json"), &omitted)?.status.success());
    Ok(())
}

#[test]
fn delta_recheck_binds_one_repair_commit_to_the_full_review_head() -> TestResult {
    let fixture = crate::support::plugin_fixture()?;
    let repo = tempfile::tempdir()?;
    init_repository(repo.path())?;
    let base = git_at(repo.path(), ["rev-parse", "HEAD"])?;
    fs::write(repo.path().join("evidence.json"), "{\"state\":\"full\"}\n")?;
    commit(repo.path(), "full")?;
    let full_head = git_at(repo.path(), ["rev-parse", "HEAD"])?;
    let ledger = repo.path().join("review-ledger.json");
    let full = packet_for(repo.path(), &base, "e-full", "full")?;
    assert!(check_packet_at(fixture.root(), repo.path(), &ledger, &full)?.status.success());
    fs::write(repo.path().join("evidence.json"), "{\"state\":\"repaired\"}\n")?;
    commit(repo.path(), "repair")?;
    let mut delta = packet_for(repo.path(), &full_head, "e-delta", "delta")?;
    delta["predecessor_event_id"] = json!("e-full");
    delta["budget"] = json!({"full_used":1,"delta_used":1});
    delta["findings"][0]["resolved"] = json!(true);
    delta["resolution"] = json!({"repaired_finding_ids":["f-1"],"changed_boundaries":["validator"]});
    delta["readiness_export"]["unresolved_blocker_ids"] = json!([]);
    delta["readiness_export"]["budget_exhausted"] = json!(true);
    assert!(check_packet_at(fixture.root(), repo.path(), &ledger, &delta)?.status.success());
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
    let dual_ledger = temp.path().join("dual.json");
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
    assert!(check_packet(fixture.root(), &dual_ledger, &valid)?.status.success());
    assert!(!check_packet(fixture.root(), &dual_ledger, &strict)?.status.success());
    for disposition in ["in_scope_nonblocking", "out_of_scope_followup"] {
        let mut observed = packet(&format!("e-{disposition}"), "full");
        observed["findings"][0]["disposition"] = json!(disposition);
        if disposition == "out_of_scope_followup" {
            observed["findings"][0]["criterion_id"] = json!(null);
            observed["findings"][0]["owned_boundary"] = json!(null);
            observed["findings"][0]["repair_boundary"] = json!(null);
        }
        observed["readiness_export"]["unresolved_blocker_ids"] = json!([]);
        assert!(check_packet(fixture.root(), &temp.path().join(format!("{disposition}.json")), &observed)?.status.success());
    }
    Ok(())
}

#[test]
fn packet_blocks_owned_defects_and_demotes_unowned_parser_scope() -> TestResult {
    let fixture = crate::support::plugin_fixture()?;
    let temp = tempfile::tempdir()?;
    for (profile, reviewer) in [
        ("standard", json!({"name":"codexy-inspector","model":"gpt-5.6-terra","reasoning_effort":"max"})),
        ("strict", json!({"name":"codexy-sentinel","model":"gpt-5.6-sol","reasoning_effort":"xhigh"})),
    ] {
        let mut owned = packet(&format!("{profile}-owned"), "full");
        set_profile(&mut owned, profile, reviewer.clone());
        owned["schema"] = json!("codexy.review-packet.v4");
        owned["findings"][0] = json!({"id":"f-1","defect_class":"bounds","criterion_id":"ac-1","owned_invariant":null,"owned_boundary":"validator","repair_boundary":"validator","counterexample":"repro","head_oid":owned["identity"]["head_oid"],"disposition":"in_scope_blocker","reopen_count":0,"resolved":false});
        assert!(check_packet(fixture.root(), &temp.path().join(format!("{profile}-owned.json")), &owned)?.status.success());

        let mut universal = owned.clone();
        universal["event_id"] = json!(format!("{profile}-universal"));
        universal["findings"][0]["criterion_id"] = json!(null);
        universal["findings"][0]["owned_invariant"] = json!(null);
        assert!(!check_packet(fixture.root(), &temp.path().join(format!("{profile}-universal.json")), &universal)?.status.success());

        let mut follow_up = universal;
        follow_up["event_id"] = json!(format!("{profile}-follow-up"));
        follow_up["findings"][0]["disposition"] = json!("out_of_scope_followup");
        follow_up["findings"][0]["owned_boundary"] = json!(null);
        follow_up["findings"][0]["repair_boundary"] = json!(null);
        follow_up["readiness_export"]["unresolved_blocker_ids"] = json!([]);
        let mut improper_repair = follow_up.clone();
        improper_repair["event_id"] = json!(format!("{profile}-improper-repair"));
        improper_repair["findings"][0]["resolved"] = json!(true);
        improper_repair["resolution"] = json!({"repaired_finding_ids":["f-1"],"changed_boundaries":["validator"]});
        assert!(!check_packet(fixture.root(), &temp.path().join(format!("{profile}-improper-repair.json")), &improper_repair)?.status.success());
        assert!(check_packet(fixture.root(), &temp.path().join(format!("{profile}-follow-up.json")), &follow_up)?.status.success());
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
    let fixture = crate::support::plugin_fixture()?; let mut valid = review_economics::report(); review_economics::bind(&mut valid, fixture.root(), &git(["rev-parse", "HEAD"]));
    assert!(check_economics(fixture.root(), &valid)?.status.success());
    for mutate in [|value: &mut Value| value["lanes"][1]["baseline_p0"] = json!(2), |value: &mut Value| value["lanes"][0]["review_ms"] = json!(50), review_economics::strict_overage] { let mut invalid = valid.clone(); mutate(&mut invalid); assert!(!check_economics(fixture.root(), &invalid)?.status.success()); }
    let unavailable = review_economics::unavailable();
    let output = check_economics(fixture.root(), &unavailable)?;
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("unavailable"));
    Ok(())
}

#[test]
fn repository_economics_result_is_readable_and_fails_closed_when_unobserved() -> TestResult {
    let fixture = crate::support::plugin_fixture()?;
    let report = fs::read_to_string(
        fixture
            .root()
            .join("skills/orchestration/references/review-economics-result.json"),
    )?;
    let output = check_economics(fixture.root(), &serde_json::from_str(&report)?)?;
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("unavailable"));
    Ok(())
}

fn packet(event: &str, state: &str) -> Value {
    let root = packet_repository();
    let base = git_at(root, ["rev-parse", "HEAD^"]).unwrap();
    packet_for(root, &base, event, state).unwrap()
}

fn packet_for(root: &Path, base: &str, event: &str, state: &str) -> TestResult<Value> {
    let head = git_at(root, ["rev-parse", "HEAD"])?;
    let range = format!("{base}..{head}");
    let diff = git_bytes_at(root, ["diff", "--no-ext-diff", "--binary", &range])?;
    let files = git_at(root, ["diff", "--name-only", "--diff-filter=ACMRD", &range])?
        .lines().map(str::to_owned).collect::<Vec<_>>();
    let evidence_path = files.first().ok_or("current test head requires a changed file")?;
    let evidence = git_bytes_at(root, ["show", &format!("{head}:{evidence_path}")])?;
    Ok(json!({"schema":"codexy.review-packet.v4","event_id":event,"predecessor_event_id":null,"profile":"standard","state":state,"reviewer":{"name":"codexy-inspector","model":"gpt-5.6-terra","reasoning_effort":"max"},"identity":{"base_oid":base,"head_oid":head,"diff_sha256":format!("{:x}",Sha256::digest(diff))},"issue_contract":{"problem":"owned problem","scope":"owned scope","acceptance_criteria":[{"id":"ac-1"}],"owned_invariant_ids":[],"exclusions":["universal parser"],"adjacent_dependencies":["typed fixture"]},"changed_files":files,"direct_boundaries":["validator"],"verification_results":[{"id":"evidence","head_oid":head,"evidence_path":evidence_path,"evidence_sha256":format!("{:x}",Sha256::digest(evidence))}],"findings":[{"id":"f-1","defect_class":"bounds","criterion_id":"ac-1","owned_invariant":null,"owned_boundary":"validator","repair_boundary":"validator","counterexample":"repro","head_oid":head,"disposition":"in_scope_blocker","reopen_count":0,"resolved":false}],"resolution":{"repaired_finding_ids":[],"changed_boundaries":[]},"budget":{"full_used":1,"delta_used":0},"readiness_export":{"head_oid":head,"profile":"standard","reviewer":{"name":"codexy-inspector","model":"gpt-5.6-terra","reasoning_effort":"max"},"unresolved_blocker_ids":["f-1"],"budget_exhausted":false,"parent_decision_required":false}}))
}

fn assert_profile(root: &Path, profile: &str, expected: Value) -> TestResult { let triggers = ["destructive","security","permission","secret","release","high_consequence_external_state","high_risk_guardrail","merge_sensitive","durable_delegation","multi_lane_ownership","explicit_audit_evidence"].into_iter().map(|kind| json!({"kind":kind,"applies":profile == "strict" && kind == "security"})).collect::<Vec<_>>(); let classification = if profile == "light" { json!({"schema":"codexy.workflow-profile-classification.v2","work_class":"low_risk","low_risk_eligible":true,"strict_triggers":triggers}) } else { json!({"schema":"codexy.workflow-profile-classification.v2","work_class":"middle","low_risk_eligible":false,"strict_triggers":triggers}) }; let request = if expected.get("discarded_lower_profile").is_some() { json!({"schema":"codexy.review-profile-request.v1","classification":classification,"prior_profile":"standard"}) } else { json!({"schema":"codexy.review-profile-request.v1","classification":classification}) }; let output = resolve_profile(root, request)?; assert!(output.status.success()); assert_eq!(serde_json::from_slice::<Value>(&output.stdout)?, expected); Ok(()) }
fn check_packet(root: &Path, ledger: &Path, value: &Value) -> TestResult<std::process::Output> { run(root, &["--repository-root", packet_repository().to_str().ok_or("root")?, "--ledger", ledger.to_str().ok_or("ledger")?, "--check-packet"], value.clone()) }
fn check_packet_at(plugin_root: &Path, repository_root: &Path, ledger: &Path, value: &Value) -> TestResult<std::process::Output> { run(plugin_root, &["--repository-root", repository_root.to_str().ok_or("root")?, "--ledger", ledger.to_str().ok_or("ledger")?, "--check-packet"], value.clone()) }
fn resolve_profile(root: &Path, value: Value) -> TestResult<std::process::Output> { run(root, &["--resolve-profile"], value) }
fn check_economics(root: &Path, value: &Value) -> TestResult<std::process::Output> { run(root, &["--check-economics"], value.clone()) }
fn run(root: &Path, flags: &[&str], value: Value) -> TestResult<std::process::Output> { let temp = tempfile::tempdir()?; let input = temp.path().join("input.json"); fs::write(&input, serde_json::to_vec(&value)?)?; Ok(Command::new(env!("CARGO_BIN_EXE_codexy-review-control")).args(["--plugin-root", root.to_str().ok_or("plugin root")?]).args(flags).args(["--input", input.to_str().ok_or("input")?]).output()?) }
fn child_routing(root: &Path, value: Value) -> TestResult<std::process::Output> { let temp = tempfile::tempdir()?; let input = temp.path().join("request.json"); fs::write(&input, serde_json::to_vec(&value)?)?; Ok(Command::new(env!("CARGO_BIN_EXE_codexy-validate")).args(["--plugin-root", root.to_str().ok_or("plugin root")?, "--resolve-child-routing", "--routing-request-file"]).arg(input).output()?) }
fn git<const N: usize>(args: [&str; N]) -> String { String::from_utf8(git_bytes(args)).unwrap().trim().to_owned() }
fn git_bytes<const N: usize>(args: [&str; N]) -> Vec<u8> { Command::new("git").current_dir(repository_root()).args(args).output().unwrap().stdout }
fn git_at<const N: usize>(root: &Path, args: [&str; N]) -> TestResult<String> { Ok(String::from_utf8(git_bytes_at(root, args)?)?.trim().to_owned()) }
fn git_bytes_at<const N: usize>(root: &Path, args: [&str; N]) -> TestResult<Vec<u8>> { let output = Command::new("git").current_dir(root).args(args).output()?; if !output.status.success() { return Err(String::from_utf8_lossy(&output.stderr).into_owned().into()); } Ok(output.stdout) }
fn init_repository(root: &Path) -> TestResult { git_at(root, ["init"])?; git_at(root, ["config", "user.email", "test@example.invalid"])?; git_at(root, ["config", "user.name", "Test"])?; fs::write(root.join("evidence.json"), "{\"state\":\"base\"}\n")?; commit(root, "base") }
fn commit(root: &Path, message: &str) -> TestResult { git_at(root, ["add", "."])?; git_at(root, ["commit", "-m", message])?; Ok(()) }
fn packet_repository() -> &'static Path { static REPOSITORY: OnceLock<tempfile::TempDir> = OnceLock::new(); REPOSITORY.get_or_init(|| { let repo = tempfile::tempdir().unwrap(); init_repository(repo.path()).unwrap(); fs::write(repo.path().join("evidence.json"), "{\"state\":\"review\"}\n").unwrap(); commit(repo.path(), "review").unwrap(); repo }).path() }
fn repository_root() -> &'static Path { codexy_runtime::paths::repository_root() }
fn too_many_blockers(value: &mut Value) { for number in 2..=4 { let mut finding = value["findings"][0].clone(); finding["id"] = json!(format!("f-{number}")); value["findings"].as_array_mut().unwrap().push(finding); } }
fn set_profile(value: &mut Value, profile: &str, reviewer: Value) { value["profile"] = json!(profile); value["reviewer"] = reviewer.clone(); value["readiness_export"]["profile"] = json!(profile); value["readiness_export"]["reviewer"] = reviewer; }
