use std::fs;

use serde_json::json;

use crate::support::TestResult;

use super::{check_packet_at, commit, git_at, init_repository, packet_for};

#[test]
fn escalated_delta_can_stop_for_parent_decision() -> TestResult {
    let fixture = crate::support::plugin_fixture()?;
    let repo = tempfile::tempdir()?;
    init_repository(repo.path())?;
    let base = git_at(repo.path(), ["rev-parse", "HEAD"])?;
    fs::write(repo.path().join("evidence.json"), "{\"state\":\"unobservable\"}\n")?;
    commit(repo.path(), "unobservable")?;
    let reviewed_head = git_at(repo.path(), ["rev-parse", "HEAD"])?;
    let ledger = repo.path().join("review-ledger.json");
    let mut unavailable = packet_for(repo.path(), &base, "e-unobservable", "unobservable")?;
    unavailable["budget"] = json!({"full_used":0,"delta_used":0});
    unavailable["findings"] = json!([]);
    unavailable["resolution"] = json!({"repaired_finding_ids":[],"changed_boundaries":[]});
    unavailable["readiness_export"]["unresolved_blocker_ids"] = json!([]);
    unavailable["readiness_export"]["budget_exhausted"] = json!(false);
    unavailable["readiness_export"]["parent_decision_required"] = json!(true);
    assert!(check_packet_at(fixture.root(), repo.path(), &ledger, &unavailable)?.status.success());

    let mut full = packet_for(repo.path(), &base, "e-strict", "full")?;
    full["profile"] = json!("strict");
    full["reviewer"] = json!({"name":"codexy-sentinel","model":"gpt-5.6-sol","reasoning_effort":"xhigh"});
    full["readiness_export"]["profile"] = json!("strict");
    full["readiness_export"]["reviewer"] = full["reviewer"].clone();
    full["predecessor_event_id"] = json!("e-unobservable");
    full["escalation"] = json!({"from_profile":"standard","predecessor_event_id":"e-unobservable","discarded_lower_profile":true});
    assert!(check_packet_at(fixture.root(), repo.path(), &ledger, &full)?.status.success());

    fs::write(repo.path().join("evidence.json"), "{\"state\":\"repair\"}\n")?;
    commit(repo.path(), "repair")?;
    let mut delta = packet_for(repo.path(), &reviewed_head, "e-delta", "delta")?;
    delta["profile"] = json!("strict");
    delta["reviewer"] = full["reviewer"].clone();
    delta["readiness_export"]["profile"] = json!("strict");
    delta["readiness_export"]["reviewer"] = delta["reviewer"].clone();
    delta["predecessor_event_id"] = json!("e-strict");
    delta["budget"] = json!({"full_used":1,"delta_used":1});
    delta["findings"][0]["resolved"] = json!(true);
    delta["resolution"] = json!({"repaired_finding_ids":["f-1"],"changed_boundaries":["validator"]});
    delta["readiness_export"]["unresolved_blocker_ids"] = json!([]);
    delta["readiness_export"]["budget_exhausted"] = json!(true);
    assert!(check_packet_at(fixture.root(), repo.path(), &ledger, &delta)?.status.success());

    let delta_head = git_at(repo.path(), ["rev-parse", "HEAD"])?;
    fs::write(repo.path().join("evidence.json"), "{\"state\":\"connector-repair\"}\n")?;
    commit(repo.path(), "connector repair")?;
    let mut decision = packet_for(repo.path(), &delta_head, "e-parent", "parent_decision")?;
    decision["profile"] = json!("strict");
    decision["reviewer"] = full["reviewer"].clone();
    decision["readiness_export"]["profile"] = json!("strict");
    decision["readiness_export"]["reviewer"] = decision["reviewer"].clone();
    decision["predecessor_event_id"] = json!("e-delta");
    decision["budget"] = json!({"full_used":1,"delta_used":1});
    decision["findings"][0]["resolved"] = json!(true);
    decision["resolution"] = json!({"repaired_finding_ids":["f-1"],"changed_boundaries":["validator"]});
    decision["readiness_export"]["unresolved_blocker_ids"] = json!([]);
    decision["readiness_export"]["budget_exhausted"] = json!(true);
    decision["readiness_export"]["parent_decision_required"] = json!(true);
    assert!(check_packet_at(fixture.root(), repo.path(), &ledger, &decision)?.status.success());

    decision["event_id"] = json!("e-detached-parent");
    decision["predecessor_event_id"] = json!("e-strict");
    assert!(!check_packet_at(fixture.root(), repo.path(), &ledger, &decision)?.status.success());
    Ok(())
}

#[test]
fn pre_cap_parent_decision_is_rejected_for_same_and_changed_heads() -> TestResult {
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

    fs::write(repo.path().join("evidence.json"), "{\"state\":\"delta\"}\n")?;
    commit(repo.path(), "delta")?;
    let mut same_head = packet_for(repo.path(), &full_head, "e-delta", "delta")?;
    same_head["predecessor_event_id"] = json!("e-full");
    same_head["budget"] = json!({"full_used":1,"delta_used":1});
    same_head["readiness_export"]["budget_exhausted"] = json!(true);
    assert!(check_packet_at(fixture.root(), repo.path(), &ledger, &same_head)?.status.success());
    let mut same_decision = same_head.clone();
    same_decision["event_id"] = json!("e-parent-same");
    same_decision["predecessor_event_id"] = json!("e-delta");
    same_decision["state"] = json!("parent_decision");
    same_decision["readiness_export"]["parent_decision_required"] = json!(true);
    assert!(!check_packet_at(fixture.root(), repo.path(), &ledger, &same_decision)?.status.success());

    let delta_head = git_at(repo.path(), ["rev-parse", "HEAD"])?;
    fs::write(repo.path().join("evidence.json"), "{\"state\":\"connector-repair\"}\n")?;
    commit(repo.path(), "connector repair")?;
    let mut changed_decision = packet_for(repo.path(), &delta_head, "e-parent-changed", "parent_decision")?;
    changed_decision["predecessor_event_id"] = json!("e-delta");
    changed_decision["budget"] = json!({"full_used":1,"delta_used":1});
    changed_decision["readiness_export"]["parent_decision_required"] = json!(true);
    assert!(!check_packet_at(fixture.root(), repo.path(), &ledger, &changed_decision)?.status.success());
    let history: serde_json::Value = serde_json::from_str(&fs::read_to_string(&ledger)?)?;
    assert_eq!(history["events"].as_array().map(Vec::len), Some(2));
    assert!(history["events"].as_array().is_some_and(|events| events.iter().all(|event| event["state"] != "parent_decision")));
    Ok(())
}

#[test]
fn post_cap_parent_decision_accepts_only_a_delta_descendant() -> TestResult {
    let fixture = crate::support::plugin_fixture()?;
    let repo = tempfile::tempdir()?;
    init_repository(repo.path())?;
    let base = git_at(repo.path(), ["rev-parse", "HEAD"])?;
    fs::write(repo.path().join("evidence.json"), "{\"state\":\"full\"}\n")?;
    commit(repo.path(), "full")?;
    let full_head = git_at(repo.path(), ["rev-parse", "HEAD"])?;
    let ledger_dir = tempfile::tempdir()?;
    let ledger = ledger_dir.path().join("review-ledger.json");
    let mut unavailable = packet_for(repo.path(), &base, "e-unobservable", "unobservable")?;
    unavailable["budget"] = json!({"full_used":0,"delta_used":0});
    unavailable["findings"] = json!([]);
    unavailable["resolution"] = json!({"repaired_finding_ids":[],"changed_boundaries":[]});
    unavailable["readiness_export"]["unresolved_blocker_ids"] = json!([]);
    unavailable["readiness_export"]["parent_decision_required"] = json!(true);
    assert!(check_packet_at(fixture.root(), repo.path(), &ledger, &unavailable)?.status.success());
    let mut full = packet_for(repo.path(), &base, "e-strict", "full")?;
    full["profile"] = json!("strict");
    full["reviewer"] = json!({"name":"codexy-sentinel","model":"gpt-5.6-sol","reasoning_effort":"xhigh"});
    full["readiness_export"]["profile"] = json!("strict");
    full["readiness_export"]["reviewer"] = full["reviewer"].clone();
    full["predecessor_event_id"] = json!("e-unobservable");
    full["escalation"] = json!({"from_profile":"standard","predecessor_event_id":"e-unobservable","discarded_lower_profile":true});
    assert!(check_packet_at(fixture.root(), repo.path(), &ledger, &full)?.status.success());
    fs::write(repo.path().join("evidence.json"), "{\"state\":\"delta\"}\n")?;
    commit(repo.path(), "delta")?;
    let delta_head = git_at(repo.path(), ["rev-parse", "HEAD"])?;
    let mut delta = packet_for(repo.path(), &full_head, "e-delta", "delta")?;
    delta["profile"] = json!("strict");
    delta["reviewer"] = full["reviewer"].clone();
    delta["readiness_export"]["profile"] = json!("strict");
    delta["readiness_export"]["reviewer"] = delta["reviewer"].clone();
    delta["predecessor_event_id"] = json!("e-strict");
    delta["budget"] = json!({"full_used":1,"delta_used":1});
    delta["findings"][0]["resolved"] = json!(true);
    delta["resolution"] = json!({"repaired_finding_ids":["f-1"],"changed_boundaries":["validator"]});
    delta["readiness_export"]["unresolved_blocker_ids"] = json!([]);
    delta["readiness_export"]["budget_exhausted"] = json!(true);
    assert!(check_packet_at(fixture.root(), repo.path(), &ledger, &delta)?.status.success());
    let sibling_ledger = ledger_dir.path().join("sibling-ledger.json");
    fs::copy(&ledger, &sibling_ledger)?;
    fs::write(repo.path().join("evidence.json"), "{\"state\":\"connector-repair\"}\n")?;
    commit(repo.path(), "connector repair")?;
    let mut decision = packet_for(repo.path(), &delta_head, "e-parent", "parent_decision")?;
    decision["profile"] = json!("strict");
    decision["reviewer"] = full["reviewer"].clone();
    decision["readiness_export"]["profile"] = json!("strict");
    decision["readiness_export"]["reviewer"] = decision["reviewer"].clone();
    decision["predecessor_event_id"] = json!("e-delta");
    decision["budget"] = json!({"full_used":1,"delta_used":1});
    decision["readiness_export"]["parent_decision_required"] = json!(true);
    decision["findings"][0]["resolved"] = json!(true);
    decision["resolution"] = json!({"repaired_finding_ids":["f-1"],"changed_boundaries":["validator"]});
    decision["readiness_export"]["unresolved_blocker_ids"] = json!([]);
    decision["readiness_export"]["budget_exhausted"] = json!(true);
    let output = check_packet_at(fixture.root(), repo.path(), &ledger, &decision)?;
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));

    git_at(repo.path(), ["checkout", full_head.as_str()])?;
    fs::write(repo.path().join("evidence.json"), "{\"state\":\"dropped-delta\"}\n")?;
    commit(repo.path(), "dropped delta")?;
    let mut sibling = packet_for(repo.path(), &delta_head, "e-sibling", "parent_decision")?;
    sibling["profile"] = json!("strict");
    sibling["reviewer"] = full["reviewer"].clone();
    sibling["readiness_export"]["profile"] = json!("strict");
    sibling["readiness_export"]["reviewer"] = sibling["reviewer"].clone();
    sibling["predecessor_event_id"] = json!("e-delta");
    sibling["budget"] = json!({"full_used":1,"delta_used":1});
    sibling["readiness_export"]["parent_decision_required"] = json!(true);
    sibling["findings"][0]["resolved"] = json!(true);
    sibling["resolution"] = json!({"repaired_finding_ids":["f-1"],"changed_boundaries":["validator"]});
    sibling["readiness_export"]["unresolved_blocker_ids"] = json!([]);
    sibling["readiness_export"]["budget_exhausted"] = json!(true);
    let output = check_packet_at(fixture.root(), repo.path(), &sibling_ledger, &sibling)?;
    assert!(!output.status.success());
    let history: serde_json::Value = serde_json::from_str(&fs::read_to_string(&sibling_ledger)?)?;
    assert_eq!(history["events"].as_array().map(Vec::len), Some(3));
    Ok(())
}
