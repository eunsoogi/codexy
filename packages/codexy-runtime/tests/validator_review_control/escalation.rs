use std::{fs, path::Path};

use serde_json::{Value, json};

use crate::support::TestResult;

use super::{check_packet_at, commit, git_at, init_repository, packet_for};

#[test]
fn parent_decision_requires_post_cap_descendant_and_delta_lineage() -> TestResult {
    let fixture = crate::support::plugin_fixture()?;
    let repo = tempfile::tempdir()?;
    init_repository(repo.path())?;
    let base = git_at(repo.path(), ["rev-parse", "HEAD"])?;
    fs::write(repo.path().join("evidence.json"), "{\"state\":\"full\"}\n")?;
    commit(repo.path(), "full")?;
    let reviewed_head = git_at(repo.path(), ["rev-parse", "HEAD"])?;
    let ledgers = tempfile::tempdir()?;
    let same_ledger = ledgers.path().join("same.json");
    let changed_ledger = ledgers.path().join("changed.json");
    let sibling_ledger = ledgers.path().join("sibling.json");
    let dropped_ledger = ledgers.path().join("dropped.json");
    let full = packet_for(repo.path(), &base, "e-full", "full")?;
    for ledger in [&same_ledger, &changed_ledger, &sibling_ledger, &dropped_ledger] {
        assert!(check_packet_at(fixture.root(), repo.path(), ledger, &full)?.status.success());
    }

    fs::write(repo.path().join("evidence.json"), "{\"state\":\"delta\"}\n")?;
    commit(repo.path(), "delta")?;
    let delta_head = git_at(repo.path(), ["rev-parse", "HEAD"])?;
    let mut delta = packet_for(repo.path(), &reviewed_head, "e-delta", "delta")?;
    delta["predecessor_event_id"] = json!("e-full");
    delta["budget"] = json!({"full_used":1,"delta_used":1});
    delta["findings"][0]["resolved"] = json!(true);
    delta["resolution"] = json!({"repaired_finding_ids":["f-1"],"changed_boundaries":["validator"]});
    delta["readiness_export"]["unresolved_blocker_ids"] = json!([]);
    delta["readiness_export"]["budget_exhausted"] = json!(true);
    for ledger in [&same_ledger, &changed_ledger, &sibling_ledger] {
        assert!(check_packet_at(fixture.root(), repo.path(), ledger, &delta)?.status.success());
    }

    let mut same_head = delta.clone();
    same_head["event_id"] = json!("e-parent-same-head");
    same_head["predecessor_event_id"] = json!("e-delta");
    same_head["state"] = json!("parent_decision");
    same_head["readiness_export"]["parent_decision_required"] = json!(true);
    assert!(!check_packet_at(fixture.root(), repo.path(), &same_ledger, &same_head)?.status.success());

    fs::write(repo.path().join("evidence.json"), "{\"state\":\"post-cap\"}\n")?;
    commit(repo.path(), "post-cap repair")?;
    let post_cap_head = git_at(repo.path(), ["rev-parse", "HEAD"])?;
    let changed_head = parent_packet(repo.path(), &delta_head, "e-parent-changed-head", "e-delta")?;
    assert!(!check_packet_at(fixture.root(), repo.path(), &changed_ledger, &changed_head)?.status.success());
    let connector_ledger = ledgers.path().join("connector.json");
    fs::copy(&changed_ledger, &connector_ledger)?;
    let mut connector_repair = changed_head.clone();
    connector_repair["event_id"] = json!("e-connector-repair");
    connector_repair["state"] = json!("connector_repair");
    connector_repair["findings"][0]["resolved"] = json!(true);
    connector_repair["resolution"] = json!({"repaired_finding_ids":["f-1"],"changed_boundaries":["validator"]});
    connector_repair["readiness_export"]["unresolved_blocker_ids"] = json!([]);
    connector_repair["readiness_export"]["parent_decision_required"] = json!(false);
    assert!(check_packet_at(fixture.root(), repo.path(), &connector_ledger, &connector_repair)?.status.success());

    let dropped = parent_packet(repo.path(), &delta_head, "e-parent-dropped-delta", "e-full")?;
    assert!(!check_packet_at(fixture.root(), repo.path(), &dropped_ledger, &dropped)?.status.success());

    git_at(repo.path(), ["switch", "--detach", &reviewed_head])?;
    fs::write(repo.path().join("evidence.json"), "{\"state\":\"sibling\"}\n")?;
    commit(repo.path(), "sibling")?;
    let sibling = parent_packet(repo.path(), &delta_head, "e-parent-sibling", "e-delta")?;
    assert!(!check_packet_at(fixture.root(), repo.path(), &sibling_ledger, &sibling)?.status.success());
    assert_ne!(post_cap_head, git_at(repo.path(), ["rev-parse", "HEAD"])?);
    Ok(())
}

fn parent_packet(root: &Path, base: &str, event: &str, predecessor: &str) -> TestResult<Value> {
    let mut packet = packet_for(root, base, event, "parent_decision")?;
    packet["predecessor_event_id"] = json!(predecessor);
    packet["budget"] = json!({"full_used":1,"delta_used":1});
    packet["readiness_export"]["budget_exhausted"] = json!(true);
    packet["readiness_export"]["parent_decision_required"] = json!(true);
    Ok(packet)
}

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

    let repair_head = git_at(repo.path(), ["rev-parse", "HEAD"])?;
    fs::write(repo.path().join("evidence.json"), "{\"state\":\"post-cap-repair\"}\n")?;
    commit(repo.path(), "post-cap repair")?;
    let mut decision = packet_for(repo.path(), &repair_head, "e-parent", "parent_decision")?;
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
    let connector_ledger = repo.path().join("connector-review-ledger.json");
    fs::copy(&ledger, &connector_ledger)?;
    let mut connector_repair = decision.clone();
    connector_repair["event_id"] = json!("e-connector-repair");
    connector_repair["state"] = json!("connector_repair");
    connector_repair["readiness_export"]["parent_decision_required"] = json!(false);
    assert!(check_packet_at(fixture.root(), repo.path(), &connector_ledger, &connector_repair)?.status.success());
    assert!(check_packet_at(fixture.root(), repo.path(), &ledger, &decision)?.status.success());

    decision["event_id"] = json!("e-detached-parent");
    decision["predecessor_event_id"] = json!("e-strict");
    assert!(!check_packet_at(fixture.root(), repo.path(), &ledger, &decision)?.status.success());
    Ok(())
}
