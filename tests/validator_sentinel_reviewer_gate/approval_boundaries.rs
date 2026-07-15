use super::*;

#[test]
fn validator_cli_rejects_sentinel_with_negated_no_finding_result_clause() -> TestResult {
    let output = validate_sentinel_replacement(
        "edge classes reviewed, replayed review examples when applicable, no-finding result when no blockers remain, and any unresolved risk",
        "MUST NOT require edge classes reviewed, replayed review examples when applicable, no-finding result when no blockers remain, and any unresolved risk",
    )?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("codexy-sentinel reviewer gate contract"));
    Ok(())
}

#[test]
fn validator_cli_rejects_sentinel_without_no_finding_result_suffix() -> TestResult {
    let output = validate_sentinel_replacement(
        "replayed review examples when applicable, no-finding result when no blockers remain, and any unresolved risk",
        "replayed review examples when applicable",
    )?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("codexy-sentinel reviewer gate contract"));
    Ok(())
}

#[test]
fn validator_cli_rejects_sentinel_with_split_negated_approval_suffix() -> TestResult {
    let output = validate_sentinel_replacement(
        "edge classes reviewed, replayed review examples when applicable, no-finding result when no blockers remain, and any unresolved risk",
        "edge classes reviewed. MUST NOT require replayed review examples when applicable, no-finding result when no blockers remain, and any unresolved risk",
    )?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("codexy-sentinel reviewer gate contract"));
    Ok(())
}

#[test]
fn validator_cli_rejects_sentinel_with_prefix_negated_approval_marker() -> TestResult {
    let output = validate_sentinel_replacement(
        "Every approval MUST reference the current diff or head",
        "MUST NOT require that Every approval MUST reference the current diff or head",
    )?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("Every approval MUST reference"));
    Ok(())
}

#[test]
fn validator_cli_rejects_sentinel_with_optional_split_approval_marker() -> TestResult {
    let output = validate_sentinel_replacement("lane scope", "lane scope is optional.")?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("lane scope"));
    Ok(())
}
