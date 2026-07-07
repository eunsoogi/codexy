use std::path::Path;
use std::process::{Command, Output};

mod support;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

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
        "Reasoning control: the packaged Sentinel definition MUST run with the highest available reasoning setting, currently model_reasoning_effort = \"xhigh\". If an invocation surface does not expose or confirm reasoning controls, the reviewer evidence MUST record explicit unavailable evidence and MUST still state that the packaged Sentinel file declares xhigh.",
        "Reasoning controls are described by later evidence expectations.",
    )?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("Reasoning control:"));
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
    let output = validate_sentinel_replacement(
        "no-finding result when no blockers remain, and any unresolved risk",
        "no-finding result when no blockers remain, and any unresolved risk\nmay be ignored",
    )?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("any unresolved risk"));
    Ok(())
}

#[test]
fn validator_cli_rejects_bare_negated_approval_marker_inside_approval_sentence() -> TestResult {
    let output = validate_sentinel_replacement(
        "reasoning control used or unavailable evidence, direct reviewer passes performed, edge classes reviewed",
        "reasoning control used or unavailable evidence, but not direct reviewer passes performed, edge classes reviewed",
    )?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("direct reviewer passes performed"));
    Ok(())
}

#[test]
fn validator_cli_rejects_sentinel_with_approval_marker_only_outside_approval_sentence() -> TestResult
{
    let output = validate_sentinel_replacement(
        "direct reviewer passes performed, edge classes reviewed, replayed review examples when applicable",
        "direct reviewer passes performed, replayed review examples when applicable",
    )?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("edge classes reviewed"));
    Ok(())
}

#[test]
fn validator_cli_accepts_wrapped_approval_evidence_sentence() -> TestResult {
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
        "For review-feedback lanes, repeated-Codex-feedback lanes, parser-heavy lanes, and validator-heavy lanes, MUST replay",
        "For review-feedback lanes, repeated-Codex-feedback lanes, parser-heavy lanes, and validator-heavy lanes, MAY skip replaying",
    )?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("repeated-Codex-feedback lanes"));
    Ok(())
}

fn validate_sentinel_replacement(needle: &str, replacement: &str) -> TestResult<Output> { validate_sentinel_edit(|sentinel| sentinel.replace(needle, replacement)) }

fn validate_sentinel_edit(edit: impl FnOnce(String) -> String) -> TestResult<Output> {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_fixture(&plugin_root)?;
    let sentinel_path = plugin_root.join("agents/codexy-sentinel.toml");
    let sentinel = std::fs::read_to_string(&sentinel_path)?;
    std::fs::write(&sentinel_path, edit(sentinel))?;
    validator(&plugin_root)
}

fn copy_fixture(plugin_root: &Path) -> std::io::Result<()> {
    support::copy_dir(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy"),
        plugin_root,
    )
}

fn validator(plugin_root: &Path) -> TestResult<Output> {
    Ok(Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--plugin-root",
            plugin_root.to_str().ok_or("plugin root path")?,
            "--check-roles",
        ])
        .output()?)
}

fn stderr(output: &Output) -> String { String::from_utf8_lossy(&output.stderr).into_owned() }