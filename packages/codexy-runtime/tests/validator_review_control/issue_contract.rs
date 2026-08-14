use std::fs;

use serde_json::json;

use super::{TestResult, check_packet, check_packet_at, commit, git_at, init_repository, packet, packet_for};

#[test]
fn packet_accepts_declared_invariants_and_rejects_undeclared_or_ambiguous_ones() -> TestResult {
    let fixture = crate::support::plugin_fixture()?;
    let temp = tempfile::tempdir()?;
    let mut declared = packet("declared-invariant", "full");
    declared["issue_contract"]["owned_invariant_ids"] = json!(["invariant-1"]);
    declared["findings"][0]["criterion_id"] = json!(null);
    declared["findings"][0]["owned_invariant"] = json!("invariant-1");
    assert!(check_packet(fixture.root(), &temp.path().join("declared.json"), &declared)?.status.success());

    let mut undeclared = declared.clone();
    undeclared["event_id"] = json!("undeclared-invariant");
    undeclared["findings"][0]["owned_invariant"] = json!("arbitrary-text");
    assert!(!check_packet(fixture.root(), &temp.path().join("undeclared.json"), &undeclared)?.status.success());

    let mut ambiguous = declared;
    ambiguous["event_id"] = json!("ambiguous-invariant");
    ambiguous["findings"][0]["criterion_id"] = json!("ac-1");
    assert!(!check_packet(fixture.root(), &temp.path().join("ambiguous.json"), &ambiguous)?.status.success());
    Ok(())
}

#[test]
fn review_cycle_requires_an_identical_issue_contract() -> TestResult {
    let fixture = crate::support::plugin_fixture()?;
    let repo = tempfile::tempdir()?;
    init_repository(repo.path())?;
    let base = git_at(repo.path(), ["rev-parse", "HEAD"])?;
    fs::write(repo.path().join("evidence.json"), "{\"state\":\"full\"}\n")?;
    commit(repo.path(), "full")?;
    let full_head = git_at(repo.path(), ["rev-parse", "HEAD"])?;
    let mut full = packet_for(repo.path(), &base, "contract-full", "full")?;
    full["issue_contract"]["owned_invariant_ids"] = json!(["invariant-1"]);
    let accepted = repo.path().join("accepted.json");
    let ledgers = ["scope", "exclusions", "invariants"]
        .map(|name| (name, repo.path().join(format!("{name}.json"))));
    assert!(check_packet_at(fixture.root(), repo.path(), &accepted, &full)?.status.success());
    for (_, ledger) in &ledgers {
        assert!(check_packet_at(fixture.root(), repo.path(), ledger, &full)?.status.success());
    }
    fs::write(repo.path().join("evidence.json"), "{\"state\":\"repair\"}\n")?;
    commit(repo.path(), "repair")?;
    let delta = delta_packet(repo.path(), &full_head)?;
    let accepted_delta = check_packet_at(fixture.root(), repo.path(), &accepted, &delta)?;
    assert!(
        accepted_delta.status.success(),
        "{}",
        String::from_utf8_lossy(&accepted_delta.stderr)
    );

    for ((name, ledger), (_, key, value)) in ledgers.into_iter().zip([
        ("scope", "scope", json!("changed scope")),
        ("exclusions", "exclusions", json!(["changed exclusion"])),
        ("invariants", "owned_invariant_ids", json!(["invariant-2"])),
    ]) {
        let mut mutated = delta.clone();
        mutated["event_id"] = json!(format!("contract-{name}"));
        mutated["issue_contract"][key] = value;
        assert!(!check_packet_at(fixture.root(), repo.path(), &ledger, &mutated)?.status.success());
    }
    Ok(())
}

fn delta_packet(root: &std::path::Path, full_head: &str) -> TestResult<serde_json::Value> {
    let mut delta = packet_for(root, full_head, "contract-delta", "delta")?;
    delta["predecessor_event_id"] = json!("contract-full");
    delta["budget"] = json!({"full_used":1,"delta_used":1});
    delta["findings"][0]["resolved"] = json!(true);
    delta["resolution"] = json!({"repaired_finding_ids":["f-1"],"changed_boundaries":["validator"]});
    delta["readiness_export"]["unresolved_blocker_ids"] = json!([]);
    delta["readiness_export"]["budget_exhausted"] = json!(true);
    delta["issue_contract"]["owned_invariant_ids"] = json!(["invariant-1"]);
    Ok(delta)
}
