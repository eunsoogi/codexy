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

    let mut decision = delta;
    decision["event_id"] = json!("e-parent");
    decision["predecessor_event_id"] = json!("e-delta");
    decision["state"] = json!("parent_decision");
    decision["readiness_export"]["parent_decision_required"] = json!(true);
    assert!(check_packet_at(fixture.root(), repo.path(), &ledger, &decision)?.status.success());

    decision["event_id"] = json!("e-detached-parent");
    decision["predecessor_event_id"] = json!("e-strict");
    assert!(!check_packet_at(fixture.root(), repo.path(), &ledger, &decision)?.status.success());
    Ok(())
}
