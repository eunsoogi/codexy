use std::fs;

use serde_json::json;

use crate::support::TestResult;

use super::{
    check_packet_at, commit, git_at, init_repository, packet_for,
};

#[test]
fn delta_and_pass_retain_each_prior_blocker_once() -> TestResult {
    let fixture = crate::support::plugin_fixture()?;
    let repo = tempfile::tempdir()?;
    init_repository(repo.path())?;
    let base = git_at(repo.path(), ["rev-parse", "HEAD"])?;
    fs::write(repo.path().join("evidence.json"), "{\"state\":\"full\"}\n")?;
    commit(repo.path(), "full")?;
    let reviewed_head = git_at(repo.path(), ["rev-parse", "HEAD"])?;
    let ledger = repo.path().join("review-ledger.json");
    let mut full = packet_for(repo.path(), &base, "e-full", "full")?;
    full["findings"][0]["reopen_count"] = json!(1);
    assert!(check_packet_at(fixture.root(), repo.path(), &ledger, &full)?.status.success());

    fs::write(repo.path().join("evidence.json"), "{\"state\":\"repaired\"}\n")?;
    commit(repo.path(), "repair")?;
    let mut delta = packet_for(repo.path(), &reviewed_head, "e-delta", "delta")?;
    delta["predecessor_event_id"] = json!("e-full");
    delta["budget"] = json!({"full_used":1,"delta_used":1});
    delta["findings"][0]["reopen_count"] = json!(1);
    delta["findings"][0]["resolved"] = json!(true);
    delta["resolution"] = json!({"repaired_finding_ids":["f-1"],"changed_boundaries":["validator"]});
    delta["readiness_export"]["unresolved_blocker_ids"] = json!([]);
    delta["readiness_export"]["budget_exhausted"] = json!(true);

    let mut dropped = delta.clone();
    dropped["event_id"] = json!("e-dropped");
    dropped["findings"] = json!([]);
    dropped["resolution"] = json!({"repaired_finding_ids":[],"changed_boundaries":[]});
    assert!(!check_packet_at(fixture.root(), repo.path(), &ledger, &dropped)?.status.success());

    let mut unrelated = delta.clone();
    unrelated["event_id"] = json!("e-unrelated");
    let mut extra = unrelated["findings"][0].clone();
    extra["id"] = json!("f-unrelated");
    unrelated["findings"].as_array_mut().ok_or("findings")?.push(extra);
    unrelated["resolution"] = json!({"repaired_finding_ids":["f-1","f-unrelated"],"changed_boundaries":["validator"]});
    assert!(!check_packet_at(fixture.root(), repo.path(), &ledger, &unrelated)?.status.success());

    let mut stale = delta.clone();
    stale["event_id"] = json!("e-stale-disposition");
    stale["findings"][0]["reopen_count"] = json!(0);
    assert!(!check_packet_at(fixture.root(), repo.path(), &ledger, &stale)?.status.success());

    assert!(check_packet_at(fixture.root(), repo.path(), &ledger, &delta)?.status.success());
    let mut passed = delta;
    passed["event_id"] = json!("e-passed");
    passed["predecessor_event_id"] = json!("e-delta");
    passed["state"] = json!("passed");
    assert!(check_packet_at(fixture.root(), repo.path(), &ledger, &passed)?.status.success());
    Ok(())
}

#[test]
fn cross_profile_review_requires_an_explicit_unobservable_escalation() -> TestResult {
    let fixture = crate::support::plugin_fixture()?;
    let temp = tempfile::tempdir()?;
    let ledger = temp.path().join("review-ledger.json");
    let mut unavailable = super::packet("e-unobservable", "unobservable");
    unavailable["budget"] = json!({"full_used":0,"delta_used":0});
    unavailable["findings"] = json!([]);
    unavailable["resolution"] = json!({"repaired_finding_ids":[],"changed_boundaries":[]});
    unavailable["readiness_export"]["unresolved_blocker_ids"] = json!([]);
    unavailable["readiness_export"]["budget_exhausted"] = json!(false);
    unavailable["readiness_export"]["parent_decision_required"] = json!(true);
    assert!(super::check_packet(fixture.root(), &ledger, &unavailable)?.status.success());

    let mut strict = super::packet("e-strict", "full");
    strict["profile"] = json!("strict");
    strict["reviewer"] = json!({"name":"codexy-sentinel","model":"gpt-5.6-sol","reasoning_effort":"xhigh"});
    strict["readiness_export"]["profile"] = json!("strict");
    strict["readiness_export"]["reviewer"] = strict["reviewer"].clone();
    assert!(!super::check_packet(fixture.root(), &ledger, &strict)?.status.success());
    strict["predecessor_event_id"] = json!("e-unobservable");
    assert!(!super::check_packet(fixture.root(), &ledger, &strict)?.status.success());
    strict["escalation"] = json!({"from_profile":"standard","predecessor_event_id":"e-unobservable","discarded_lower_profile":true});
    assert!(super::check_packet(fixture.root(), &ledger, &strict)?.status.success());
    Ok(())
}

#[test]
fn review_cycle_requires_the_tip_and_stops_same_class_replays() -> TestResult {
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

    fs::write(repo.path().join("evidence.json"), "{\"state\":\"repair-one\"}\n")?;
    commit(repo.path(), "repair-one")?;
    let mut delta = packet_for(repo.path(), &full_head, "e-delta", "delta")?;
    delta["predecessor_event_id"] = json!("e-full");
    delta["budget"] = json!({"full_used":1,"delta_used":1});
    delta["readiness_export"]["budget_exhausted"] = json!(true);
    assert!(check_packet_at(fixture.root(), repo.path(), &ledger, &delta)?.status.success());
    let repair_one = git_at(repo.path(), ["rev-parse", "HEAD"])?;

    fs::write(repo.path().join("evidence.json"), "{\"state\":\"repair-two\"}\n")?;
    commit(repo.path(), "repair-two")?;
    let mut branched_delta = packet_for(repo.path(), &full_head, "e-branched", "delta")?;
    branched_delta["predecessor_event_id"] = json!("e-full");
    branched_delta["budget"] = json!({"full_used":1,"delta_used":1});
    branched_delta["readiness_export"]["budget_exhausted"] = json!(true);
    let branched = check_packet_at(fixture.root(), repo.path(), &ledger, &branched_delta)?.status.success();

    let restarted = packet_for(repo.path(), &repair_one, "e-restarted", "full")?;
    let restarted = check_packet_at(fixture.root(), repo.path(), &ledger, &restarted)?.status.success();
    assert!(!branched && !restarted);

    Ok(())
}

#[test]
fn delta_rejects_a_new_unresolved_blocker_in_the_same_defect_class() -> TestResult {
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
    fs::write(repo.path().join("evidence.json"), "{\"state\":\"repair\"}\n")?;
    commit(repo.path(), "repair")?;
    let mut delta = packet_for(repo.path(), &full_head, "e-delta", "delta")?;
    delta["predecessor_event_id"] = json!("e-full");
    delta["budget"] = json!({"full_used":1,"delta_used":1});
    delta["readiness_export"]["budget_exhausted"] = json!(true);
    let mut repeated = delta["findings"][0].clone();
    repeated["id"] = json!("f-2");
    repeated["counterexample"] = json!("same defect class");
    delta["findings"].as_array_mut().ok_or("findings")?.push(repeated);
    delta["readiness_export"]["unresolved_blocker_ids"] = json!(["f-1", "f-2"]);
    assert!(!check_packet_at(fixture.root(), repo.path(), &ledger, &delta)?.status.success());
    Ok(())
}

#[test]
fn clean_full_review_can_transition_to_passed() -> TestResult {
    let fixture = crate::support::plugin_fixture()?;
    let temp = tempfile::tempdir()?;
    let ledger = temp.path().join("review-ledger.json");
    let mut full = super::packet("e-clean-full", "full");
    full["findings"] = json!([]);
    full["resolution"] = json!({"repaired_finding_ids":[],"changed_boundaries":[]});
    full["readiness_export"]["unresolved_blocker_ids"] = json!([]);
    assert!(super::check_packet(fixture.root(), &ledger, &full)?.status.success());
    let mut passed = full;
    passed["event_id"] = json!("e-clean-passed");
    passed["predecessor_event_id"] = json!("e-clean-full");
    passed["state"] = json!("passed");
    assert!(super::check_packet(fixture.root(), &ledger, &passed)?.status.success());
    Ok(())
}
