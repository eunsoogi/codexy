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
