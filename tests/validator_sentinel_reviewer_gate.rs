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
fn validator_cli_rejects_negated_reasoning_control_evidence() -> TestResult {
    for replacement in [
        "missing reasoning control used or unavailable evidence is acceptable",
        "reasoning control used or unavailable evidence is optional",
        "reasoning control used or unavailable evidence: optional",
        "reasoning control used or unavailable evidence, optional",
        "optional: reasoning control used or unavailable evidence",
        "no reasoning control used or unavailable evidence is required",
        "no reasoning control used or unavailable evidence required",
        "no explicit reasoning control used or unavailable evidence is required",
        "does not require reasoning control used or unavailable evidence",
        "doesn't need reasoning control used or unavailable evidence",
        "waived reasoning control used or unavailable evidence",
        "reasoning control used or unavailable evidence may be skipped",
        "reasoning control used or unavailable evidence may\nbe skipped",
        "reasoning control used or unavailable evidence does not have to be supplied",
        "reasoning control used or unavailable evidence does not\nneed to be supplied",
        "reasoning control used or unavailable evidence needn't be supplied",
        "reasoning control used or unavailable evidence isn't required",
        "reasoning control used or unavailable evidence isn't necessary",
        "reasoning control used or unavailable evidence is not necessary",
        "reasoning control used or unavailable evidence is not explicitly required",
        "reasoning control used or unavailable evidence may be left out",
        "may omit reasoning control used or unavailable evidence",
        "may\nomit reasoning control used or unavailable evidence",
        "can omit reasoning control used or unavailable evidence",
        "reasoning control used or unavailable evidence\nis optional",
        "reasoning control used or unavailable evidence. This evidence is optional",
        "reasoning control used or unavailable evidence. However, this evidence is optional",
        "reasoning control used or unavailable evidence. But this requirement may be skipped",
        "reasoning control used or unavailable evidence. Also, this evidence is optional",
        "reasoning control used or unavailable evidence. Additionally, this evidence may be skipped",
        "reasoning control used or unavailable evidence. In practice, this evidence is waived",
        "reasoning control used or unavailable evidence. Still, this requirement is optional",
        "reasoning control used or unavailable evidence. Nevertheless, this evidence may be skipped",
        "reasoning control used or unavailable evidence. May be skipped",
        "reasoning control used or unavailable evidence. Optional for reviewers",
        "reasoning control used or unavailable evidence. Waived in practice",
        "reasoning control used or unavailable evidence. Not required for this gate",
        "reasoning control used or unavailable evidence. Reviewers may skip this evidence",
        "reasoning control used or unavailable evidence. This requirement may be skipped",
        "reasoning control used or unavailable evidence. This evidence is waived",
        "MUST NOT record reasoning control used or unavailable evidence",
        "recording reasoning control used or unavailable evidence is forbidden",
        "recording reasoning control used or unavailable evidence is prohibited",
    ] {
        let output = validate_sentinel_replacement(
            "reasoning control used or unavailable evidence",
            replacement,
        )?;

        assert!(
            !output.status.success(),
            "validator accepted {replacement:?}"
        );
        assert!(stderr(&output).contains("reasoning-control evidence must be affirmative"));
    }
    Ok(())
}

#[test]
fn validator_cli_rejects_non_affirmative_reasoning_control_paragraph() -> TestResult {
    for replacement in [
        "Reasoning control: the packaged Sentinel definition MUST run with the highest available reasoning setting, currently model_reasoning_effort = \"xhigh\" is optional. Reviewer evidence MUST record explicit unavailable evidence.\n\n",
        "No Reasoning control: the packaged Sentinel definition MUST run with the highest available reasoning setting, currently model_reasoning_effort = \"xhigh\". Reviewer evidence MUST record explicit unavailable evidence.\n\n",
        "Negated Reasoning control: the packaged Sentinel definition MUST run with the highest available reasoning setting, currently model_reasoning_effort = \"xhigh\". Reviewer evidence MUST record explicit unavailable evidence.\n\n",
        "Reasoning control: no packaged Sentinel definition MUST run with the highest available reasoning setting, currently model_reasoning_effort = \"xhigh\". Reviewer evidence MUST record explicit unavailable evidence.\n\n",
        "Reasoning control: the packaged Sentinel definition MUST run with the highest available reasoning setting, currently model_reasoning_effort = \"xhigh\". Reviewer evidence MUST record explicit unavailable evidence is forbidden.\n\n",
    ] {
        let output = validate_reasoning_control_paragraph_replacement(replacement)?;

        assert!(
            !output.status.success(),
            "validator accepted {replacement:?}"
        );
        assert!(stderr(&output).contains("reasoning-control paragraph must be present"));
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
