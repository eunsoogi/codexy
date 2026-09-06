use super::*;

#[test]
fn eligibility_rejects_connector_provenance_and_identity_changes() -> TestResult {
    let temporary = tempfile::tempdir()?;
    let base = "1111111111111111111111111111111111111111";
    let head = "2222222222222222222222222222222222222222";
    let valid = connector_snapshot::source_only(&direct_state::pr_snapshot(947, base, head, None));
    let mut unauthenticated = valid.clone();
    unauthenticated["capture"]["authenticated"] = json!(false);
    let mut contradiction = valid.clone();
    contradiction["number"] = json!(948);
    let mut missing_source = valid.clone();
    missing_source["capture"].as_object_mut().unwrap().remove("source");
    let mut missing_capture = valid.clone();
    missing_capture.as_object_mut().unwrap().remove("capture");
    let input = temporary.path().join("request.json");
    fs::write(&input, serde_json::to_vec(&json!({
        "authenticated_finding_disposition_locator": {}
    }))?)?;
    for (invalid, diagnostic) in [
        (unauthenticated, "not authenticated"),
        (contradiction, "PR identity"),
        (missing_source, "source provenance"),
        (missing_capture, "must contain repository"),
    ] {
        for current_invalid in [true, false] {
            let current = temporary.path().join("current.json");
            let previous = temporary.path().join("previous.json");
            fs::write(&current, serde_json::to_vec(if current_invalid { &invalid } else { &valid })?)?;
            fs::write(&previous, serde_json::to_vec(if current_invalid { &valid } else { &invalid })?)?;
            let output = temporary.path().join("output.json");
            let result = std::process::Command::new(env!("CARGO_BIN_EXE_codexy-review-control"))
                .arg("--check-next-review-eligibility")
                .arg("--input").arg(&input)
                .arg("--current-pr-state-file").arg(&current)
                .arg("--previous-pr-state-file").arg(&previous)
                .arg("--output").arg(&output).output()?;
            assert!(!result.status.success());
            assert!(String::from_utf8_lossy(&result.stderr).contains(diagnostic));
            assert!(!output.exists());
        }
    }
    Ok(())
}
