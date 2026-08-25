use std::fs;

use serde_json::{Value, json};

#[path = "validator_review_handoff/contracts.rs"] mod contracts;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn completion_handoff_requires_typed_selected_profile_evidence() -> TestResult {
    assert!(!validate(None)?.status.success());
    assert!(validate(Some("light"))?.status.success());
    assert!(!validate(Some("standard"))?.status.success());
    assert!(!validate(Some("strict"))?.status.success());
    for evidence in [
        r#"{"schema":"codexy.review-readiness.v1","head_oid":"stale","profile":"standard","reviewer":{"name":"codexy-inspector","model":"gpt-5.6-terra","reasoning_effort":"max"},"state":"passed"}"#,
        r#"{"schema":"codexy.review-readiness.v1","head_oid":"h","profile":"unknown","reviewer":{"name":"codexy-inspector","model":"gpt-5.6-terra","reasoning_effort":"max"},"state":"passed"}"#,
        r#"{"schema":"codexy.review-readiness.v1","head_oid":"h","profile":"standard","reviewer":null,"state":"passed"}"#,
        r#"{"schema":"codexy.review-readiness.v1","head_oid":"h","profile":"standard","reviewer":{"name":"codexy-sentinel","model":"gpt-5.6-sol","reasoning_effort":"xhigh"},"state":"passed"}"#,
    ] {
        assert!(!validate_evidence(evidence)?.status.success());
    }
    Ok(())
}

#[test]
fn completion_handoff_binds_the_terminal_event_of_its_review_ledger() -> TestResult {
    assert!(validate_bound(|_| {})?.status.success());
    for mutate in [
        |state: &mut Value| state["reviewEvidence"]["event_id"] = json!("other"),
        |state: &mut Value| state["reviewLedger"]["events"][0]["head_oid"] = json!("stale"),
        |state: &mut Value| state["reviewLedger"]["events"][0]["state"] = json!("delta"),
        |state: &mut Value| state["reviewLedger"]["events"][1]["base_oid"] = json!("other"),
        |state: &mut Value| state["reviewLedger"]["events"][1]["boundaries"] = json!(["other"]),
        |state: &mut Value| { state["reviewLedger"]["events"].as_array_mut().expect("events").remove(0); },
    ] {
        assert!(!validate_bound(mutate)?.status.success());
    }
    Ok(())
}

#[test]
fn completion_handoff_rejects_missing_or_empty_typed_review_identity() -> TestResult {
    assert!(!validate_bound(|state| {
        state["headRefOid"] = json!("");
        state["reviewEvidence"]["head_oid"] = json!("");
        for event in state["reviewLedger"]["events"].as_array_mut().expect("events") {
            event["head_oid"] = json!("");
        }
    })?.status.success());
    assert!(!validate_bound(|state| { state.as_object_mut().expect("state").remove("reviewLedger"); })?.status.success());
    assert!(!validate_bound(|state| state["reviewProfile"] = json!("light"))?.status.success());
    Ok(())
}

#[test]
fn completion_handoff_accepts_the_recordable_escalated_delta_cycle() -> TestResult {
    assert!(validate_escalated_delta(|_| {})?.status.success());
    Ok(())
}

#[test]
fn completion_handoff_rejects_a_sibling_post_cap_parent_decision() -> TestResult {
    assert!(!validate_escalated_parent_decision(|_| {})?.status.success());
    for mutate in [
        |state: &mut Value| state["reviewLedger"]["events"][3]["predecessor_event_id"] = json!("e-strict"),
        |state: &mut Value| state["reviewLedger"]["events"][3]["head_oid"] = json!("stale"),
        |state: &mut Value| state["reviewLedger"]["events"][3]["base_oid"] = json!("other"),
        |state: &mut Value| { state["reviewLedger"]["events"][2]["blockers"] = json!([{"id":"f-1","defect_class":"bounds","resolved":false,"reopen_count":0}]); state["reviewLedger"]["events"][3]["blockers"] = state["reviewLedger"]["events"][2]["blockers"].clone(); state["reviewEvidence"]["blockers"] = state["reviewLedger"]["events"][3]["blockers"].clone(); },
    ] {
        assert!(!validate_escalated_parent_decision(mutate)?.status.success());
    }
    Ok(())
}

#[test]
fn completion_handoff_rejects_invalid_cycle_event_ids() -> TestResult {
    for mutate in [
        |state: &mut Value| {
            state["reviewLedger"]["events"][0]["id"] = json!("");
            state["reviewLedger"]["events"][1]["predecessor_event_id"] = json!("");
        },
        |state: &mut Value| {
            state["reviewLedger"]["events"][0]["id"] = json!("not valid");
            state["reviewLedger"]["events"][1]["predecessor_event_id"] = json!("not valid");
        },
        |state: &mut Value| {
            state["reviewLedger"]["events"][0]["id"] = json!("e-passed");
            state["reviewLedger"]["events"][1]["predecessor_event_id"] = json!("e-passed");
        },
    ] {
        assert!(!validate_bound(mutate)?.status.success());
    }
    Ok(())
}

#[test]
fn completion_handoff_rejects_a_nonlight_zero_review_cycle() -> TestResult {
    assert!(!validate_bound(|state| {
        state["reviewLedger"]["events"].as_array_mut().expect("events").remove(0);
        state["reviewLedger"]["events"][0]["predecessor_event_id"] = Value::Null;
        state["reviewLedger"]["events"][0]["full_used"] = json!(0);
        state["reviewLedger"]["events"][0]["delta_used"] = json!(0);
    })?.status.success());
    Ok(())
}

#[test]
fn completion_handoff_binds_delta_to_the_preceding_full_base() -> TestResult {
    assert!(validate_delta_base(|_| {})?.status.success());
    for mutate in [
        |state: &mut Value| {
            state["reviewLedger"]["events"][1]
                .as_object_mut()
                .expect("delta")
                .remove("base_oid");
        },
        |state: &mut Value| state["reviewLedger"]["events"][1]["base_oid"] = json!("older"),
        |state: &mut Value| state["reviewLedger"]["events"][1]["base_oid"] = json!(""),
    ] {
        assert!(!validate_delta_base(mutate)?.status.success());
    }
    Ok(())
}

fn validate(profile: Option<&str>) -> TestResult<std::process::Output> {
    let temp = tempfile::tempdir()?;
    let handoff = temp.path().join("handoff.md");
    let state_path = temp.path().join("state.json");
    fs::write(&handoff, "Maintainer requested leave-open; implementation complete.\n")?;
    let evidence: String = profile.map_or("null".into(), |profile| match profile {
        "standard" => r#"{"schema":"codexy.review-readiness.v1","head_oid":"h","profile":"standard","reviewer":{"name":"codexy-inspector","model":"gpt-5.6-terra","reasoning_effort":"max"},"state":"passed"}"#.into(),
        _ => r#"{"schema":"codexy.review-readiness.v1","head_oid":"h","profile":"strict","reviewer":{"name":"codexy-inspector","model":"gpt-5.6-terra","reasoning_effort":"max"},"state":"passed"}"#.into(),
    });
    fs::write(&state_path, state_json(&evidence, profile))?;
    crate::support::validator_completion_handoff_files(&handoff, &state_path)
}

fn validate_evidence(evidence: &str) -> TestResult<std::process::Output> {
    let temp = tempfile::tempdir()?;
    let handoff = temp.path().join("handoff.md");
    let state = temp.path().join("state.json");
    fs::write(&handoff, "Maintainer requested leave-open; implementation complete.\n")?;
    fs::write(&state, state_json(evidence, Some("standard")))?;
    crate::support::validator_completion_handoff_files(&handoff, &state)
}

fn state_json(evidence: &str, profile: Option<&str>) -> String {
    let decision = if profile == Some("light") { "NOT_REQUIRED" } else { "APPROVED" };
    let profile = profile.map_or("null".to_owned(), |value| format!("\"{value}\""));
    let mut state: Value = serde_json::from_str(&format!(r#"{{"state":"OPEN","isDraft":true,"mergeStateStatus":"CLEAN","headRefOid":"h","reviewDecision":"{decision}","reviewProfile":{profile},"reviewEvidence":{evidence}}}"#)).expect("state");
    if decision == "NOT_REQUIRED" {
        state
            .as_object_mut()
            .expect("state")
            .remove("reviewEvidence");
    }
    crate::support::review_control_state::namespace_review_control(&mut state);
    serde_json::to_string(&state).expect("state JSON")
}

fn validate_bound(mutate: impl FnOnce(&mut Value)) -> TestResult<std::process::Output> {
    let temp = tempfile::tempdir()?;
    let handoff = temp.path().join("handoff.md");
    let state_path = temp.path().join("state.json");
    fs::write(&handoff, "Maintainer requested leave-open; implementation complete.\n")?;
    let mut state = json!({
        "state":"OPEN", "isDraft":true, "mergeStateStatus":"CLEAN", "headRefOid":"h", "reviewDecision":"APPROVED",
        "reviewProfile":"standard",
        "reviewEvidence":{"schema":"codexy.review-readiness.v1","head_oid":"h","profile":"standard","reviewer":{"name":"codexy-inspector","model":"gpt-5.6-terra","reasoning_effort":"max"},"state":"passed","event_id":"e-passed","blockers":[]},
        "reviewLedger":{"schema":"codexy.review-ledger.v1","events":[{"id":"e-full","predecessor_event_id":null,"profile":"standard","base_oid":"base","head_oid":"h","state":"full","full_used":1,"delta_used":0,"blockers":[],"boundaries":["validator"],"escalation":null},{"id":"e-passed","predecessor_event_id":"e-full","profile":"standard","base_oid":"base","head_oid":"h","state":"passed","full_used":1,"delta_used":0,"blockers":[],"boundaries":["validator"],"escalation":null}]}
    });
    contracts::bind_ledger(&mut state);
    mutate(&mut state);
    crate::support::review_control_state::namespace_review_control(&mut state);
    fs::write(&state_path, serde_json::to_vec(&state)?)?;
    crate::support::validator_completion_handoff_files(&handoff, &state_path)
}

fn validate_escalated_delta(
    mutate: impl FnOnce(&mut Value),
) -> TestResult<std::process::Output> {
    let temp = tempfile::tempdir()?;
    let handoff = temp.path().join("handoff.md");
    let state_path = temp.path().join("state.json");
    fs::write(&handoff, "Maintainer requested leave-open; implementation complete.\n")?;
    let mut state = json!({
        "state":"OPEN", "isDraft":true, "mergeStateStatus":"CLEAN", "headRefOid":"repair", "reviewDecision":"APPROVED",
        "reviewProfile":"strict",
        "reviewEvidence":{"schema":"codexy.review-readiness.v1","head_oid":"repair","profile":"strict","reviewer":{"name":"codexy-sentinel","model":"gpt-5.6-sol","reasoning_effort":"xhigh"},"state":"passed","event_id":"e-passed","blockers":[]},
        "reviewLedger":{"schema":"codexy.review-ledger.v1","events":[
            {"id":"e-unobservable","predecessor_event_id":null,"profile":"standard","base_oid":"base","head_oid":"reviewed","state":"unobservable","full_used":0,"delta_used":0,"blockers":[],"boundaries":["validator"],"escalation":null},
            {"id":"e-strict","predecessor_event_id":"e-unobservable","profile":"strict","base_oid":"base","head_oid":"reviewed","state":"full","full_used":1,"delta_used":0,"blockers":[],"boundaries":["validator"],"escalation":{"from_profile":"standard","predecessor_event_id":"e-unobservable","discarded_lower_profile":true}},
            {"id":"e-delta","predecessor_event_id":"e-strict","profile":"strict","base_oid":"reviewed","head_oid":"repair","state":"delta","full_used":1,"delta_used":1,"blockers":[],"boundaries":["validator"],"escalation":null},
            {"id":"e-passed","predecessor_event_id":"e-delta","profile":"strict","base_oid":"reviewed","head_oid":"repair","state":"passed","full_used":1,"delta_used":1,"blockers":[],"boundaries":["validator"],"escalation":null}
        ]}
    });
    contracts::bind_ledger(&mut state);
    mutate(&mut state);
    crate::support::review_control_state::namespace_review_control(&mut state);
    fs::write(&state_path, serde_json::to_vec(&state)?)?;
    crate::support::validator_completion_handoff_files(&handoff, &state_path)
}

fn validate_escalated_parent_decision(
    mutate: impl FnOnce(&mut Value),
) -> TestResult<std::process::Output> {
    let temp = tempfile::tempdir()?;
    let handoff = temp.path().join("handoff.md");
    let state_path = temp.path().join("state.json");
    fs::write(&handoff, "Maintainer requested leave-open; implementation complete.\n")?;
    let mut state = json!({
        "state":"OPEN", "isDraft":true, "mergeStateStatus":"CLEAN", "headRefOid":"467ef39c2ada7d0f64daca2dc9bca833aa1fb00c", "reviewDecision":"PARENT_DECISION",
        "reviewProfile":"strict",
        "reviewEvidence":{"schema":"codexy.review-readiness.v1","head_oid":"467ef39c2ada7d0f64daca2dc9bca833aa1fb00c","profile":"strict","reviewer":{"name":"codexy-sentinel","model":"gpt-5.6-sol","reasoning_effort":"xhigh"},"state":"parent_decision","event_id":"e-parent","blockers":[]},
        "reviewLedger":{"schema":"codexy.review-ledger.v1","events":[
            {"id":"e-unobservable","predecessor_event_id":null,"profile":"standard","base_oid":"base","head_oid":"reviewed","state":"unobservable","full_used":0,"delta_used":0,"blockers":[],"boundaries":["validator"],"escalation":null},
            {"id":"e-strict","predecessor_event_id":"e-unobservable","profile":"strict","base_oid":"base","head_oid":"reviewed","state":"full","full_used":1,"delta_used":0,"blockers":[],"boundaries":["validator"],"escalation":{"from_profile":"standard","predecessor_event_id":"e-unobservable","discarded_lower_profile":true}},
            {"id":"e-delta","predecessor_event_id":"e-strict","profile":"strict","base_oid":"reviewed","head_oid":"bb954a46d06d0fc2c8ebb8009f6e8f835703f71c","state":"delta","full_used":1,"delta_used":1,"blockers":[],"boundaries":["validator"],"escalation":null},
            {"id":"e-parent","predecessor_event_id":"e-delta","profile":"strict","base_oid":"reviewed","head_oid":"467ef39c2ada7d0f64daca2dc9bca833aa1fb00c","state":"parent_decision","full_used":1,"delta_used":1,"blockers":[],"boundaries":["validator"],"escalation":null}
        ]}
    });
    contracts::bind_ledger(&mut state);
    mutate(&mut state);
    crate::support::review_control_state::namespace_review_control(&mut state);
    fs::write(&state_path, serde_json::to_vec(&state)?)?;
    crate::support::validator_completion_handoff_files(&handoff, &state_path)
}

fn validate_delta_base(mutate: impl FnOnce(&mut Value)) -> TestResult<std::process::Output> {
    let temp = tempfile::tempdir()?;
    let handoff = temp.path().join("handoff.md");
    let state_path = temp.path().join("state.json");
    fs::write(&handoff, "Maintainer requested leave-open; implementation complete.\n")?;
    let mut state = json!({
        "state":"OPEN", "isDraft":true, "mergeStateStatus":"CLEAN", "headRefOid":"repair", "reviewDecision":"APPROVED",
        "reviewProfile":"standard",
        "reviewEvidence":{"schema":"codexy.review-readiness.v1","head_oid":"repair","profile":"standard","reviewer":{"name":"codexy-inspector","model":"gpt-5.6-terra","reasoning_effort":"max"},"state":"passed","event_id":"e-passed","blockers":[]},
        "reviewLedger":{"schema":"codexy.review-ledger.v1","events":[
            {"id":"e-full","predecessor_event_id":null,"profile":"standard","base_oid":"base","head_oid":"reviewed","state":"full","full_used":1,"delta_used":0,"blockers":[],"boundaries":["validator"],"escalation":null},
            {"id":"e-delta","predecessor_event_id":"e-full","profile":"standard","base_oid":"reviewed","head_oid":"repair","state":"delta","full_used":1,"delta_used":1,"blockers":[],"boundaries":["validator"],"escalation":null},
            {"id":"e-passed","predecessor_event_id":"e-delta","profile":"standard","base_oid":"reviewed","head_oid":"repair","state":"passed","full_used":1,"delta_used":1,"blockers":[],"boundaries":["validator"],"escalation":null}
        ]}
    });
    contracts::bind_ledger(&mut state);
    mutate(&mut state);
    crate::support::review_control_state::namespace_review_control(&mut state);
    fs::write(&state_path, serde_json::to_vec(&state)?)?;
    crate::support::validator_completion_handoff_files(&handoff, &state_path)
}
