use super::*;

#[test]
fn compact_review_control_accepts_current_head_without_history_or_quota() -> TestResult {
    let control = compact_control("PASS", json!([]));
    let temp = tempfile::tempdir()?;
    let (base, control_path, previous_path, output) = state_files(temp.path(), &control)?;
    fs::write(&previous_path, b"historical transcript unavailable")?;
    let result = run_capture(&base, &control_path, &previous_path, &output)?;
    assert!(
        result.status.success(),
        "current-head review must not require old history: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    let state: Value = serde_json::from_slice(&fs::read(output)?)?;
    assert_eq!(state["reviewControl"], control);
    Ok(())
}

#[test]
fn compact_review_control_accepts_without_previous_state_file() -> TestResult {
    let temp = tempfile::tempdir()?;
    let current = temp.path().join("current-pr-state.json");
    let control_path = temp.path().join("review-control.json");
    let output = temp.path().join("pr-state.json");
    let control = compact_control("PASS", json!([]));
    fs::write(
        &current,
        serde_json::to_vec(&direct_state::pr_snapshot(725, BASE_OID, HEAD_OID, None))?,
    )?;
    fs::write(&control_path, serde_json::to_vec(&control)?)?;
    let result = FixtureCommand::new(
        codexy_runtime::paths::repository_root().join("scripts/build-pr-state"),
    )
    .arg("--base-pr-state-file")
    .arg_path(&current)
    .arg("--review-control-state-file")
    .arg_path(&control_path)
    .arg("--output")
    .arg_path(&output)
    .env_path(
        "CODEXY_REVIEW_CONTROL_BIN",
        env!("CARGO_BIN_EXE_codexy-review-control"),
    )
    .output()?;
    assert!(
        result.status.success(),
        "compact review control must not require a previous state file: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    Ok(())
}

#[test]
fn compact_review_control_keeps_actual_findings_blocking() -> TestResult {
    let output = validate_readiness_with(
        compact_control("PASS", json!([{"id": "finding-1"}])),
        "LOC remediation: focused check passed.\n",
        json!({}),
    )?;
    assert!(
        !output.status.success(),
        "actual unresolved findings must still block readiness"
    );
    Ok(())
}

#[test]
fn compact_review_control_rejects_legacy_provenance_without_legacy_state() -> TestResult {
    let mut control = compact_control("PASS", json!([]));
    control["native_history_provenance"] = json!({"schema": "legacy"});
    let output = validate_readiness(control)?;
    assert!(
        !output.status.success(),
        "legacy provenance must not be admitted as compact current-head state"
    );
    Ok(())
}

#[test]
fn compact_review_control_rejects_legacy_recovery_without_legacy_state() -> TestResult {
    let mut control = compact_control("PASS", json!([]));
    control["native_history_recovery"] = json!({"schema": "legacy"});
    let output = validate_readiness(control)?;
    assert!(
        !output.status.success(),
        "legacy recovery must not be admitted as compact current-head state"
    );
    Ok(())
}

#[test]
fn compact_review_control_rejects_ambiguous_terminal_and_pending_state() -> TestResult {
    let mut control = compact_control("PASS", json!([]));
    control["status"] = json!("RUNNING");
    let output = validate_readiness(control)?;
    assert!(
        !output.status.success(),
        "a compact state must not claim terminal and pending results together"
    );
    Ok(())
}

fn compact_control(result: &str, findings: Value) -> Value {
    json!({
        "schema": "codexy.review-control-state.v1",
        "profile": "strict",
        "reviewer": {
            "name": "codexy-sentinel",
            "model": "gpt-6-astra",
            "reasoning_effort": "xhigh"
        },
        "reviewed_head": HEAD_OID,
        "terminal_result": result,
        "unresolved_findings": findings
    })
}
