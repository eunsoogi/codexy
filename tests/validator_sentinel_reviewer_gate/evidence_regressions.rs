use super::*;

#[test]
fn validator_cli_rejects_weakened_marker_inside_approval_sentence_with_external_copy() -> TestResult
{
    let output = validate_sentinel_edit(|sentinel| {
        let external_markers = "\n\nAudit vocabulary: Every approval MUST reference the current diff or head, lane scope, touched implementation-file LOC evidence, verification commands and results, direct readback for structured files, reasoning control used or unavailable evidence, direct reviewer passes performed, edge classes reviewed, replayed review examples when applicable, no-finding result when no blockers remain, and any unresolved risk.";
        let anchor = "MUST block when negative tests are absent for validator, parser, guardrail, workflow-rule, or review-feedback fixes unless the lane proves why negative coverage is not applicable.";
        sentinel.replace(anchor, &format!("{anchor}{external_markers}")).replacen(
            "direct reviewer passes performed, edge classes reviewed, replayed review examples when applicable",
            "direct reviewer passes performed, edge classes reviewed is optional, replayed review examples when applicable",
            1,
        )
    })?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("edge classes reviewed"));
    Ok(())
}

#[test]
fn validator_cli_rejects_weakened_real_approval_sentence_after_external_copy() -> TestResult {
    let output = validate_sentinel_edit(|sentinel| {
        let external_markers = "Audit vocabulary: Every approval MUST reference the current diff or head, lane scope, touched implementation-file LOC evidence, verification commands and results, direct readback for structured files, reasoning control used or unavailable evidence, direct reviewer passes performed, edge classes reviewed, replayed review examples when applicable, no-finding result when no blockers remain, and any unresolved risk.\n\nEvidence expectations:";
        let target = "direct reviewer passes performed, edge classes reviewed, replayed review examples when applicable";
        let mut sentinel = sentinel
            .replace("Evidence expectations:", external_markers)
            .replace(
                "touched implementation-file LOC evidence when applicable",
                "touched implementation-file LOC evidence",
            );
        let first = sentinel.find(target).expect("external approval marker");
        let second = first
            + target.len()
            + sentinel[first + target.len()..]
                .find(target)
                .expect("real approval marker");
        sentinel.replace_range(
            second..second + target.len(),
            "direct reviewer passes performed, replayed review examples",
        );
        sentinel
    })?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("edge classes reviewed"));
    Ok(())
}

#[test]
fn validator_cli_rejects_ignored_approval_marker_inside_approval_sentence() -> TestResult {
    let output = validate_sentinel_replacement(
        "reasoning control used or unavailable evidence, direct reviewer passes performed, edge classes reviewed",
        "reasoning control used or unavailable evidence, direct reviewer passes performed may be ignored, edge classes reviewed",
    )?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("direct reviewer passes performed"));
    for replacement in [
        "reasoning control used or unavailable evidence, but not direct reviewer passes performed, edge classes reviewed",
        "reasoning control used or unavailable evidence, no direct reviewer passes performed, edge classes reviewed",
        "reasoning control used or unavailable evidence, no\n direct reviewer passes performed, edge classes reviewed",
        "reasoning control used or unavailable evidence, not direct reviewer passes performed, edge classes reviewed",
        "reasoning control used or unavailable evidence, direct reviewer passes performed is waived, edge classes reviewed",
        "reasoning control used or unavailable evidence, direct reviewer passes performed is not needed, edge classes reviewed",
        "reasoning control used or unavailable evidence, direct reviewer passes performed when applicable, edge classes reviewed if available",
        "reasoning control used or unavailable evidence, direct reviewer passes performed, when applicable, edge classes reviewed",
    ] {
        let output = validate_sentinel_replacement(
            "reasoning control used or unavailable evidence, direct reviewer passes performed, edge classes reviewed",
            replacement,
        )?;
        assert!(!output.status.success());
        assert!(stderr(&output).contains("direct reviewer passes performed"));
    }
    let output = validate_sentinel_replacement(
        "reasoning control used or unavailable evidence, direct reviewer passes performed, edge classes reviewed",
        "reasoning control used or unavailable evidence, direct reviewer passes performed, edge classes reviewed. Direct reviewer passes performed may be skipped",
    )?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("replayed review examples when applicable"));
    let output = validate_sentinel_replacement(
        "no-finding result when no blockers remain, and any unresolved risk",
        "no-finding result when no blockers remain, and any unresolved risk\nmay be ignored",
    )?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("any unresolved risk"));
    Ok(())
}

#[test]
fn validator_cli_rejects_sentinel_with_approval_marker_only_outside_approval_sentence() -> TestResult
{
    let output = validate_sentinel_edit(|sentinel| {
        let anchor = "MUST block when negative tests are absent for validator, parser, guardrail, workflow-rule, or review-feedback fixes unless the lane proves why negative coverage is not applicable.";
        sentinel.replacen("direct reviewer passes performed, edge classes reviewed, replayed review examples when applicable", "direct reviewer passes performed, replayed review examples when applicable", 1).replace(anchor, &format!("{anchor} Edge classes reviewed."))
    })?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("edge classes reviewed"));
    Ok(())
}

#[test]
fn validator_cli_accepts_wrapped_approval_evidence_sentence() -> TestResult {
    let output = validate_sentinel_edit(|sentinel| sentinel)?;
    assert!(output.status.success(), "{}", stderr(&output));
    let output = validate_sentinel_replacement(
        "Every approval MUST reference the current diff or head, lane scope",
        "Every approval MUST reference the current diff or head,\n lane scope",
    )?;
    assert!(output.status.success(), "{}", stderr(&output));
    Ok(())
}

#[test]
fn validator_cli_rejects_sentinel_with_weakened_review_example_replay() -> TestResult {
    let output = validate_sentinel_replacement(
        "For review-feedback lanes, repeated-automated-feedback lanes, parser-heavy lanes, and validator-heavy lanes, MUST replay",
        "For review-feedback lanes, repeated-automated-feedback lanes, parser-heavy lanes, and validator-heavy lanes, MAY skip replaying",
    )?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("repeated-automated-feedback lanes"));
    Ok(())
}
