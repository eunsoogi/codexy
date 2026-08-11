use std::fs;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn completion_handoff_requires_typed_selected_profile_evidence() -> TestResult {
    assert!(!validate(None)?.status.success());
    assert!(validate_light()?.status.success());
    assert!(validate(Some("standard"))?.status.success());
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

fn validate(profile: Option<&str>) -> TestResult<std::process::Output> {
    let temp = tempfile::tempdir()?;
    let handoff = temp.path().join("handoff.md");
    let state = temp.path().join("state.json");
    fs::write(&handoff, "Maintainer requested leave-open; implementation complete.\n")?;
    let evidence: String = profile.map_or("null".into(), |profile| match profile {
        "standard" => r#"{"schema":"codexy.review-readiness.v1","head_oid":"h","profile":"standard","reviewer":{"name":"codexy-inspector","model":"gpt-5.6-terra","reasoning_effort":"max"},"state":"passed"}"#.into(),
        _ => r#"{"schema":"codexy.review-readiness.v1","head_oid":"h","profile":"strict","reviewer":{"name":"codexy-inspector","model":"gpt-5.6-terra","reasoning_effort":"max"},"state":"passed"}"#.into(),
    });
    fs::write(&state, state_json(&evidence, profile))?;
    crate::support::validator_completion_handoff_files(&handoff, &state)
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
