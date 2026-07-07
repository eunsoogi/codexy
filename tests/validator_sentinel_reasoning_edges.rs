use std::path::Path;
use std::process::{Command, Output};

mod support;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn validator_cli_rejects_additional_no_longer_reasoning_opt_outs() -> TestResult {
    for replacement in [
        "reasoning control used or unavailable evidence is no longer necessary",
        "reasoning control used or unavailable evidence is no longer needed",
        "reasoning control used or unavailable evidence is no longer mandatory",
    ] {
        assert_reasoning_evidence_rejected(validate_sentinel_replacement(
            "reasoning control used or unavailable evidence",
            replacement,
        )?)?;
    }
    Ok(())
}

#[test]
fn validator_cli_rejects_pre_marker_reasoning_conditionals() -> TestResult {
    for replacement in [
        "Every approval MUST, when the reviewer can confirm it, reference reasoning control used or unavailable evidence",
        "Every approval MUST, if the reviewer can confirm it, reference reasoning control used or unavailable evidence",
        "Every approval MUST, whenever feasible, reference reasoning control used or unavailable evidence",
    ] {
        assert_reasoning_evidence_rejected(validate_sentinel_replacement(
            "reasoning control used or unavailable evidence",
            replacement,
        )?)?;
    }
    Ok(())
}

#[test]
fn validator_cli_rejects_generic_evidence_opt_outs_after_marker() -> TestResult {
    assert_reasoning_evidence_rejected(validate_sentinel_replacement(
        "reasoning control used or unavailable evidence, direct reviewer passes performed",
        "reasoning control used or unavailable evidence, but the reviewer need not supply evidence, direct reviewer passes performed",
    )?)
}

#[test]
fn validator_cli_rejects_trailing_conditional_reasoning_opt_outs() -> TestResult {
    for replacement in [
        "reasoning control used or unavailable evidence, which is required if the reviewer can confirm it",
        "reasoning control used or unavailable evidence is required when tools expose it",
    ] {
        assert_reasoning_evidence_rejected(validate_sentinel_replacement(
            "reasoning control used or unavailable evidence",
            replacement,
        )?)?;
    }
    Ok(())
}

#[test]
fn validator_cli_handles_unicode_before_reasoning_marker() -> TestResult {
    let output = validate_sentinel_edit(|sentinel| {
        Ok(sentinel
            .replace(
                "direct readback for structured files, reasoning control used",
                "direct readback for structured files — reasoning control used",
            )
            .replace(
                "reasoning control used or unavailable evidence",
                "reasoning control used or unavailable evidence is optional",
            ))
    })?;
    assert_reasoning_evidence_rejected(output)
}

fn assert_reasoning_evidence_rejected(output: Output) -> TestResult {
    let stderr = stderr(&output);
    assert!(!output.status.success(), "{stderr}");
    assert!(!stderr.contains("panicked at"), "{stderr}");
    assert!(stderr.contains("reasoning-control evidence must be affirmative"));
    Ok(())
}

fn validate_sentinel_replacement(needle: &str, replacement: &str) -> TestResult<Output> {
    validate_sentinel_edit(|sentinel| Ok(sentinel.replace(needle, replacement)))
}

fn validate_sentinel_edit(edit: impl FnOnce(String) -> TestResult<String>) -> TestResult<Output> {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    support::copy_dir(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy"),
        &plugin_root,
    )?;
    let sentinel_path = plugin_root.join("agents/codexy-sentinel.toml");
    let sentinel = std::fs::read_to_string(&sentinel_path)?;
    std::fs::write(&sentinel_path, edit(sentinel)?)?;
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
