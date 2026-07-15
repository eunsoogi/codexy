use super::*;

#[test]
fn validator_cli_rejects_sentinel_without_xhigh_reasoning() -> TestResult {
    let output = validate_sentinel_replacement(
        "model_reasoning_effort = \"xhigh\"",
        "model_reasoning_effort = \"high\"",
    )?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("codexy-sentinel model_reasoning_effort must be xhigh"));
    Ok(())
}

#[test]
fn validator_cli_rejects_sentinel_without_specialized_review_passes() -> TestResult {
    let output =
        validate_sentinel_replacement("validator/parser edge-case pass", "edge-case pass")?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("codexy-sentinel reviewer gate contract"));
    Ok(())
}

#[test]
fn validator_cli_rejects_sentinel_without_reasoning_control_marker() -> TestResult {
    let output = validate_sentinel_replacement(
        "Reasoning control: the packaged Sentinel definition MUST use the deliberate high-intensity reviewer setting model_reasoning_effort = \"xhigh\" alongside model = \"gpt-5.6-sol\". It MUST NOT claim or require max or ultra. If an invocation surface does not expose or confirm reasoning controls, the reviewer evidence MUST record explicit unavailable evidence and MUST still state that the packaged Sentinel file declares xhigh.",
        "Reasoning controls are described by later evidence expectations.",
    )?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("Reasoning control:"));
    Ok(())
}

#[test]
fn sentinel_reasoning_contract_is_deliberately_xhigh_not_runtime_maximum() -> TestResult {
    let sentinel_path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy/agents/codexy-sentinel.toml");
    let sentinel = std::fs::read_to_string(sentinel_path)?;

    assert!(sentinel.contains(
        "Reasoning control: the packaged Sentinel definition MUST use the deliberate high-intensity reviewer setting model_reasoning_effort = \"xhigh\" alongside model = \"gpt-5.6-sol\""
    ));
    assert!(!sentinel.contains("highest available reasoning setting"));
    assert!(!sentinel.contains("runtime maximum"));
    assert!(sentinel.contains("MUST NOT claim or require max or ultra"));
    Ok(())
}

#[test]
fn validator_cli_rejects_sentinel_with_negated_specialized_review_passes() -> TestResult {
    let output = validate_sentinel_replacement(
        "Reviewer specialization: MUST split the review into named passes",
        "Reviewer specialization: MUST NOT split the review into named passes",
    )?;
    assert!(!output.status.success());
    assert!(
        stderr(&output)
            .contains("Reviewer specialization: MUST split the review into named passes")
    );
    Ok(())
}

#[test]
fn validator_cli_rejects_sentinel_without_reasoning_unavailable_evidence_clause() -> TestResult {
    let output = validate_sentinel_replacement(
        "the reviewer evidence MUST record explicit unavailable evidence",
        "the reviewer evidence can omit unavailable evidence",
    )?;
    assert!(!output.status.success());
    assert!(
        stderr(&output).contains("the reviewer evidence MUST record explicit unavailable evidence")
    );
    Ok(())
}
