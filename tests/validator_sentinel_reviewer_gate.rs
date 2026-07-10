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
fn validator_cli_rejects_sentinel_without_reasoning_control_paragraph() -> TestResult {
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

#[test]
fn validator_cli_rejects_sentinel_with_negated_no_finding_result_clause() -> TestResult {
    let output = validate_sentinel_replacement(
        "edge classes reviewed, replayed review examples when applicable, no-finding result when no blockers remain, and any unresolved risk",
        "MUST NOT require edge classes reviewed, replayed review examples when applicable, no-finding result when no blockers remain, and any unresolved risk",
    )?;

    assert!(!output.status.success());
    assert!(stderr(&output).contains("edge classes reviewed"));
    Ok(())
}

#[test]
fn validator_cli_rejects_sentinel_without_no_finding_result_suffix() -> TestResult {
    let output = validate_sentinel_replacement(
        "replayed review examples when applicable, no-finding result when no blockers remain, and any unresolved risk",
        "replayed review examples when applicable",
    )?;

    assert!(!output.status.success());
    assert!(stderr(&output).contains("no-finding result when no blockers remain"));
    Ok(())
}

#[test]
fn validator_cli_rejects_sentinel_with_split_negated_approval_suffix() -> TestResult {
    let output = validate_sentinel_replacement(
        "edge classes reviewed, replayed review examples when applicable, no-finding result when no blockers remain, and any unresolved risk",
        "edge classes reviewed. MUST NOT require replayed review examples when applicable, no-finding result when no blockers remain, and any unresolved risk",
    )?;

    assert!(!output.status.success());
    assert!(stderr(&output).contains("Every approval MUST reference"));
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
fn validator_cli_rejects_sentinel_with_weakened_review_example_replay() -> TestResult {
    let output = validate_sentinel_replacement(
        "For review-feedback lanes, repeated-Codex-feedback lanes, parser-heavy lanes, and validator-heavy lanes, MUST replay",
        "For review-feedback lanes, repeated-Codex-feedback lanes, parser-heavy lanes, and validator-heavy lanes, MAY skip replaying",
    )?;

    assert!(!output.status.success());
    assert!(stderr(&output).contains("repeated-Codex-feedback lanes"));
    Ok(())
}

fn validate_sentinel_replacement(needle: &str, replacement: &str) -> TestResult<Output> {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_fixture(&plugin_root)?;
    let sentinel_path = plugin_root.join("agents/codexy-sentinel.toml");
    let sentinel = std::fs::read_to_string(&sentinel_path)?;
    std::fs::write(&sentinel_path, sentinel.replace(needle, replacement))?;
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

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
