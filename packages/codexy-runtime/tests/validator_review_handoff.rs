use std::fs;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn completion_handoff_requires_typed_selected_profile_evidence() -> TestResult {
    assert!(!validate(None)?.status.success());
    assert!(validate(Some("standard"))?.status.success());
    assert!(!validate(Some("strict"))?.status.success());
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
    fs::write(&state, format!(r#"{{"state":"OPEN","isDraft":true,"mergeStateStatus":"CLEAN","headRefOid":"h","reviewEvidence":{evidence}}}"#))?;
    crate::support::validator_completion_handoff_files(&handoff, &state)
}
