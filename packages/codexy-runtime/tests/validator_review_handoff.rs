use std::fs;

use serde_json::{Value, json};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn completion_handoff_requires_typed_selected_profile_evidence() -> TestResult {
    assert!(!validate(None)?.status.success());
    assert!(validate_light()?.status.success());
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
        |state: &mut Value| { state["reviewLedger"]["events"].as_array_mut().expect("events").remove(0); },
    ] {
        assert!(!validate_bound(mutate)?.status.success());
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

fn validate_light() -> TestResult<std::process::Output> {
    let temp = tempfile::tempdir()?;
    let handoff = temp.path().join("handoff.md");
    let state = temp.path().join("state.json");
    fs::write(&handoff, "Maintainer requested leave-open; implementation complete.\n")?;
    fs::write(
        &state,
        r#"{"state":"OPEN","isDraft":true,"mergeStateStatus":"CLEAN","headRefOid":"h","reviewProfile":"light"}"#,
    )?;
    crate::support::validator_completion_handoff_files(&handoff, &state)
}

fn state_json(evidence: &str, profile: Option<&str>) -> String {
    let profile = profile.map_or("null".to_owned(), |value| format!("\"{value}\""));
    format!(r#"{{"state":"OPEN","isDraft":true,"mergeStateStatus":"CLEAN","headRefOid":"h","reviewProfile":{profile},"reviewEvidence":{evidence}}}"#)
}

fn validate_bound(mutate: impl FnOnce(&mut Value)) -> TestResult<std::process::Output> {
    let temp = tempfile::tempdir()?;
    let handoff = temp.path().join("handoff.md");
    let state_path = temp.path().join("state.json");
    fs::write(&handoff, "Maintainer requested leave-open; implementation complete.\n")?;
    let mut state = json!({
        "state":"OPEN", "isDraft":true, "mergeStateStatus":"CLEAN", "headRefOid":"h",
        "reviewProfile":"standard",
        "reviewEvidence":{"schema":"codexy.review-readiness.v1","head_oid":"h","profile":"standard","reviewer":{"name":"codexy-inspector","model":"gpt-5.6-terra","reasoning_effort":"max"},"state":"passed","event_id":"e-passed","blockers":[]},
        "reviewLedger":{"schema":"codexy.review-ledger.v1","events":[{"id":"e-full","predecessor_event_id":null,"profile":"standard","head_oid":"h","state":"full","full_used":1,"delta_used":0,"blockers":[],"boundaries":["validator"],"escalation":null},{"id":"e-passed","predecessor_event_id":"e-full","profile":"standard","head_oid":"h","state":"passed","full_used":1,"delta_used":0,"blockers":[],"boundaries":["validator"],"escalation":null}]}
    });
    mutate(&mut state);
    fs::write(&state_path, serde_json::to_vec(&state)?)?;
    crate::support::validator_completion_handoff_files(&handoff, &state_path)
}
