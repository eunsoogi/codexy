use std::path::Path;
use std::process::{Command, Output};

mod support;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn validator_cli_rejects_weak_reasoning_control_evidence_preamble() -> TestResult {
    const EVIDENCE_PREAMBLE: &str = "Every approval MUST reference the current diff or head";
    for replacement in [
        "Every approval MUST try to reference the current diff or head",
        "Every approval MUST attempt to reference the current diff or head",
        "Every approval MUST prefer to reference the current diff or head",
        "Every approval MUST endeavor to reference the current diff or head",
        "Every approval MUST strive to reference the current diff or head",
        "Every approval MUST make reasonable efforts to reference the current diff or head",
    ] {
        let output = validate_sentinel_replacement(EVIDENCE_PREAMBLE, replacement)?;
        assert!(!output.status.success(), "accepted {replacement:?}");
        assert!(stderr(&output).contains("reasoning-control evidence must be affirmative"));
    }
    Ok(())
}
#[test]
fn validator_cli_accepts_affirmative_reasoning_control_evidence_control() -> TestResult {
    let output = validate_sentinel_replacement(
        "reasoning control used or unavailable evidence",
        "reasoning control used or unavailable evidence remains required",
    )?;
    assert!(output.status.success(), "{}", stderr(&output));
    Ok(())
}

#[test]
fn validator_cli_rejects_weakened_affirmative_reference_clauses() -> TestResult {
    for replacement in [
        "reasoning control used or unavailable evidence, MUST reference if available direct reviewer passes performed",
        "reasoning control used or unavailable evidence, MUST record, when available, direct reviewer passes performed",
        "reasoning control used or unavailable evidence, MUST reference if the evidence is available direct reviewer passes performed",
        "reasoning control used or unavailable evidence, MUST record, when it is applicable, direct reviewer passes performed",
        "reasoning control used or unavailable evidence, MUST reference,\noptionally,\ndirect reviewer passes performed",
        "reasoning control used or unavailable evidence, MUST reference only if available direct reviewer passes performed",
        "reasoning control used or unavailable evidence, MUST record — unless waived — direct reviewer passes performed",
    ] {
        let output = validate_sentinel_replacement(
            "reasoning control used or unavailable evidence, direct reviewer passes performed",
            replacement,
        )?;
        assert!(!output.status.success(), "accepted {replacement:?}");
    }
    Ok(())
}

#[test]
fn validator_cli_accepts_unweakened_affirmative_reference_clauses() -> TestResult {
    for replacement in [
        "reasoning control used or unavailable evidence, MUST reference direct reviewer passes performed",
        "reasoning control used or unavailable evidence, MUST\nrecord\ndirect reviewer passes performed",
    ] {
        let output = validate_sentinel_replacement(
            "reasoning control used or unavailable evidence, direct reviewer passes performed",
            replacement,
        )?;
        assert!(
            output.status.success(),
            "{replacement}: {}",
            stderr(&output)
        );
    }
    Ok(())
}

#[test]
fn validator_cli_rejects_negated_later_approval_evidence() -> TestResult {
    for replacement in [
        "reasoning control used or unavailable evidence, but not direct reviewer passes performed",
        "MUST NOT omit reasoning control used or unavailable evidence, and MUST reference direct reviewer passes performed is waived",
    ] {
        let output = validate_sentinel_replacement(
            "reasoning control used or unavailable evidence, direct reviewer passes performed",
            replacement,
        )?;
        assert!(!output.status.success(), "accepted {replacement:?}");
        assert!(stderr(&output).contains("reviewer gate contract is missing"));
    }
    Ok(())
}

#[test]
fn validator_cli_rejects_non_affirmative_reasoning_control_paragraph() -> TestResult {
    for replacement in [
        "Reasoning control: the packaged Sentinel definition MUST use the deliberate high-intensity reviewer setting model_reasoning_effort = \"xhigh\" alongside model = \"gpt-5.6-sol\". It MUST NOT claim or require max or ultra is optional. Reviewer evidence MUST record explicit unavailable evidence.\n\n",
        "No Reasoning control: the packaged Sentinel definition MUST use the deliberate high-intensity reviewer setting model_reasoning_effort = \"xhigh\" alongside model = \"gpt-5.6-sol\". It MUST NOT claim or require max or ultra. Reviewer evidence MUST record explicit unavailable evidence.\n\n",
        "Negated Reasoning control: the packaged Sentinel definition MUST use the deliberate high-intensity reviewer setting model_reasoning_effort = \"xhigh\" alongside model = \"gpt-5.6-sol\". It MUST NOT claim or require max or ultra. Reviewer evidence MUST record explicit unavailable evidence.\n\n",
        "Not Reasoning control: the packaged Sentinel definition MUST use the deliberate high-intensity reviewer setting model_reasoning_effort = \"xhigh\" alongside model = \"gpt-5.6-sol\". It MUST NOT claim or require max or ultra. Reviewer evidence MUST record explicit unavailable evidence.\n\n",
        "Reasoning control: no packaged Sentinel definition MUST use the deliberate high-intensity reviewer setting model_reasoning_effort = \"xhigh\" alongside model = \"gpt-5.6-sol\". It MUST NOT claim or require max or ultra. Reviewer evidence MUST record explicit unavailable evidence.\n\n",
        "Reasoning control: the packaged Sentinel definition MUST use the deliberate high-intensity reviewer setting model_reasoning_effort = \"xhigh\" alongside model = \"gpt-5.6-sol\". It MUST NOT claim or require max or ultra. Reviewer evidence MUST record explicit unavailable evidence is forbidden.\n\n",
    ] {
        let output = validate_reasoning_control_paragraph_replacement(replacement)?;
        assert!(!output.status.success(), "accepted {replacement:?}");
        assert!(stderr(&output).contains("reasoning-control paragraph must be present"));
    }
    Ok(())
}
#[test]
fn validator_cli_accepts_affirmative_no_surface_reasoning_control_paragraph() -> TestResult {
    for replacement in [
        "Reasoning control: the packaged Sentinel definition MUST use the deliberate high-intensity reviewer setting model_reasoning_effort = \"xhigh\" alongside model = \"gpt-5.6-sol\". It MUST NOT claim or require max or ultra. If an invocation surface is available without reasoning controls, the reviewer evidence MUST record explicit unavailable evidence.\n\n",
        "Reasoning control: the packaged Sentinel definition MUST use the deliberate high-intensity reviewer setting model_reasoning_effort = \"xhigh\" alongside model = \"gpt-5.6-sol\". It MUST NOT claim or require max or ultra. If the invocation surface is missing reasoning controls, the reviewer evidence MUST record explicit unavailable evidence.\n\n",
    ] {
        let output = validate_reasoning_control_paragraph_replacement(replacement)?;
        assert!(output.status.success(), "{}", stderr(&output));
    }
    Ok(())
}
#[test]
fn validator_cli_rejects_sentinel_without_reasoning_control_paragraph() -> TestResult {
    let output = validate_sentinel_edit(|mut sentinel| {
        let start = sentinel
            .find("Reasoning control:")
            .ok_or("reasoning control paragraph start")?;
        let end = sentinel
            .find("Adversarial review method:")
            .ok_or("reasoning control paragraph end")?;
        sentinel.replace_range(start..end, "");
        Ok(sentinel)
    })?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("reasoning-control paragraph must be present"));
    Ok(())
}
fn validate_sentinel_replacement(needle: &str, replacement: &str) -> TestResult<Output> {
    validate_sentinel_edit(|sentinel| Ok(sentinel.replace(needle, replacement)))
}
fn validate_reasoning_control_paragraph_replacement(replacement: &str) -> TestResult<Output> {
    validate_sentinel_edit(|mut sentinel| {
        let start = sentinel
            .find("Reasoning control:")
            .ok_or("reasoning control paragraph start")?;
        let end = sentinel
            .find("Adversarial review method:")
            .ok_or("reasoning control paragraph end")?;
        sentinel.replace_range(start..end, replacement);
        Ok(sentinel)
    })
}

fn validate_sentinel_edit(edit: impl FnOnce(String) -> TestResult<String>) -> TestResult<Output> {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_fixture(&plugin_root)?;
    let sentinel_path = plugin_root.join("agents/codexy-sentinel.toml");
    let sentinel = std::fs::read_to_string(&sentinel_path)?;
    std::fs::write(&sentinel_path, edit(sentinel)?)?;
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
