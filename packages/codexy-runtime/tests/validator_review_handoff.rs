use std::fs;

use serde_json::{Value, json};

#[path = "validator_review_handoff/contracts.rs"] mod contracts;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[path = "validator_review_handoff/evidence_cases.rs"]
mod evidence_cases;
#[path = "validator_review_handoff/ledger_cases.rs"]
mod ledger_cases;

fn validate(profile: Option<&str>) -> TestResult<std::process::Output> {
    let temp = tempfile::tempdir()?;
    let handoff = temp.path().join("handoff.md");
    let state_path = temp.path().join("state.json");
    fs::write(
        &handoff,
        "Maintainer requested leave-open; implementation complete.\n",
    )?;
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
    fs::write(
        &handoff,
        "Maintainer requested leave-open; implementation complete.\n",
    )?;
    fs::write(&state, state_json(evidence, Some("standard")))?;
    crate::support::validator_completion_handoff_files(&handoff, &state)
}

fn state_json(evidence: &str, profile: Option<&str>) -> String {
    let decision = if profile == Some("light") {
        "NOT_REQUIRED"
    } else {
        "APPROVED"
    };
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
    fs::write(
        &handoff,
        "Maintainer requested leave-open; implementation complete.\n",
    )?;
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

fn validate_escalated_delta(mutate: impl FnOnce(&mut Value)) -> TestResult<std::process::Output> {
    let temp = tempfile::tempdir()?;
    let handoff = temp.path().join("handoff.md");
    let state_path = temp.path().join("state.json");
    fs::write(
        &handoff,
        "Maintainer requested leave-open; implementation complete.\n",
    )?;
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
    fs::write(
        &handoff,
        "Maintainer requested leave-open; implementation complete.\n",
    )?;
    let mut state = json!({
        "state":"OPEN", "isDraft":true, "mergeStateStatus":"CLEAN", "headRefOid":"repair", "reviewDecision":"PARENT_DECISION",
        "reviewProfile":"strict",
        "reviewEvidence":{"schema":"codexy.review-readiness.v1","head_oid":"repair","profile":"strict","reviewer":{"name":"codexy-sentinel","model":"gpt-5.6-sol","reasoning_effort":"xhigh"},"state":"parent_decision","event_id":"e-parent","blockers":[]},
        "reviewLedger":{"schema":"codexy.review-ledger.v1","events":[
            {"id":"e-unobservable","predecessor_event_id":null,"profile":"standard","base_oid":"base","head_oid":"reviewed","state":"unobservable","full_used":0,"delta_used":0,"blockers":[],"boundaries":["validator"],"escalation":null},
            {"id":"e-strict","predecessor_event_id":"e-unobservable","profile":"strict","base_oid":"base","head_oid":"reviewed","state":"full","full_used":1,"delta_used":0,"blockers":[],"boundaries":["validator"],"escalation":{"from_profile":"standard","predecessor_event_id":"e-unobservable","discarded_lower_profile":true}},
            {"id":"e-delta","predecessor_event_id":"e-strict","profile":"strict","base_oid":"reviewed","head_oid":"repair","state":"delta","full_used":1,"delta_used":1,"blockers":[],"boundaries":["validator"],"escalation":null},
            {"id":"e-parent","predecessor_event_id":"e-delta","profile":"strict","base_oid":"reviewed","head_oid":"repair","state":"parent_decision","full_used":1,"delta_used":1,"blockers":[],"boundaries":["validator"],"escalation":null}
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
    fs::write(
        &handoff,
        "Maintainer requested leave-open; implementation complete.\n",
    )?;
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
