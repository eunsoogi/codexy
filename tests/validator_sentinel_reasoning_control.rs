use std::path::Path;
use std::process::{Command, Output};

mod support;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn validator_cli_rejects_negated_reasoning_control_evidence() -> TestResult {
    for replacement in [
        "missing reasoning control used or unavailable evidence is acceptable",
        "reasoning control used or unavailable evidence is optional",
        "no reasoning control used or unavailable evidence is required",
        "no reasoning control used or unavailable evidence required",
        "no explicit reasoning control used or unavailable evidence is required",
        "does not require reasoning control used or unavailable evidence",
        "doesn't need reasoning control used or unavailable evidence",
        "waived reasoning control used or unavailable evidence",
        "reasoning control used or unavailable evidence may be skipped",
        "reasoning control used or unavailable evidence may\nbe skipped",
        "Every approval SHOULD reference reasoning control used or unavailable evidence",
        "Every approval can reference reasoning control used or unavailable evidence",
        "Every approval MUST reference reasoning control used or unavailable evidence if available",
        "Every approval MUST reference reasoning control used or unavailable evidence if applicable",
        "Every approval MUST reference reasoning control used or unavailable evidence, if applicable",
        "Every approval MUST reference reasoning control used or unavailable evidence, but reviewers may omit it",
        "Every approval MUST reference reasoning control used or unavailable evidence when needed",
        "Every approval MUST reference reasoning control used or unavailable evidence when feasible",
        "Every approval MUST reference reasoning control used or unavailable evidence when possible",
        "Every approval MUST, if applicable, reference reasoning control used or unavailable evidence",
        "Every approval MUST, when applicable, reference reasoning control used or unavailable evidence",
        "Every approval MUST, where applicable, reference reasoning control used or unavailable evidence",
        "Every approval MUST, as applicable, reference reasoning control used or unavailable evidence",
        "Every approval MUST reference, when applicable, reasoning control used or unavailable evidence",
        "Every approval MUST reference reasoning control used or unavailable evidence only if requested",
        "Every approval MUST reference reasoning control used or unavailable evidence provided that the reviewer can confirm it",
        "Every approval MUST reference reasoning control used or unavailable evidence subject to tool availability",
        "Every approval MUST reference reasoning control used or unavailable evidence unless the invocation surface exposes no reasoning controls",
        "Every approval MUST reference reasoning control used or unavailable evidence except when the invocation surface exposes no reasoning controls",
        "Every approval MUST reference reasoning control used or unavailable evidence except if the invocation surface exposes no reasoning controls",
        "Every approval MUST reference reasoning control used or unavailable evidence where applicable",
        "Every approval MUST reference reasoning control used or unavailable evidence as applicable",
        "reasoning control used or unavailable evidence as needed",
        "reasoning control used or unavailable evidence where practical",
        "Every approval MUST consider reasoning control used or unavailable evidence",
        "Every approval MUST inspect reasoning control used or unavailable evidence",
        "reasoning control used or unavailable evidence does not have to be supplied",
        "reasoning control used or unavailable evidence does not\nneed to be supplied",
        "reasoning control used or unavailable evidence needn't be supplied",
        "reasoning control used or unavailable evidence isn't required",
        "reasoning control used or unavailable evidence isn't necessary",
        "reasoning control used or unavailable evidence is not explicitly required",
        "reasoning control used or unavailable evidence is no longer required",
        "reasoning control used or unavailable evidence is never required",
        "reasoning control used or unavailable evidence is not obligatory",
        "reasoning control used or unavailable evidence is not expected",
        "reasoning control used or unavailable evidence is for awareness only",
        "reasoning control used or unavailable evidence is encouraged",
        "reasoning control used or unavailable evidence is suggested",
        "reasoning control used or unavailable evidence is voluntary",
        "reasoning control used or unavailable evidence should be recorded",
        "reasoning control used or unavailable evidence can be disregarded",
        "may omit reasoning control used or unavailable evidence",
        "may\nomit reasoning control used or unavailable evidence",
        "reasoning control used or unavailable evidence may be disregarded",
        "reasoning control used or unavailable evidence\nis optional",
        "reasoning control used or unavailable evidence. This evidence is optional",
        "reasoning control used or unavailable evidence. However, this evidence is optional",
        "reasoning control used or unavailable evidence. Reviewers are allowed to ignore it",
        "reasoning control used or unavailable evidence. Reviewers are permitted to ignore it",
        "reasoning control used or unavailable evidence. Reviewers are allowed to disregard it",
        "reasoning control used or unavailable evidence. In practice, this evidence is waived",
        "reasoning control used or unavailable evidence is not binding",
        "Every approval MUST, at reviewer discretion, reference reasoning control used or unavailable evidence",
        "reasoning control used or unavailable evidence, this evidence records the invocation surface, but reviewers may omit it",
        "reasoning control used or unavailable evidence, but reviewers may leave it out, direct reviewer passes performed",
        "reasoning control used or unavailable evidence. Reviewers may choose not to include it. Direct reviewer passes performed",
        "reasoning control used or unavailable evidence. Not required for this gate",
        "reasoning control used or unavailable evidence. The reviewer may omit this",
        "reasoning control used or unavailable evidence. Reviewers may skip it",
        "reasoning control used or unavailable evidence. Reviewers may ignore it",
        "reasoning control used or unavailable evidence. Reviewers can decide whether to include it.",
        "reasoning control used or unavailable evidence. Reviewers can choose whether to include it.",
        "reasoning control used or unavailable evidence. It is at the reviewer's discretion.",
        "reasoning control used or unavailable evidence. This requirement may be skipped",
        "reasoning control used or unavailable evidence. This evidence is waived",
        "reasoning control used or unavailable evidence. This evidence records the invocation surface. It may be omitted",
        "Every approval MUST never record reasoning control used or unavailable evidence",
        "Every approval MUST-NOT record reasoning control used or unavailable evidence",
        "MUST NOT record reasoning control used or unavailable evidence",
        "recording reasoning control used or unavailable evidence is forbidden",
        "reasoning control used or unavailable evidence is not in any practical sense required",
        "reasoning control used or unavailable evidence is best-effort",
        "reasoning control used or unavailable evidence except in rare cases",
        "reasoning control used or unavailable evidence only for merge readiness",
        "reasoning control used or unavailable evidence except, in rare cases",
        "reasoning control used or unavailable evidence only, for merge readiness",
        "reasoning control used or unavailable evidence except-in rare cases",
        "reasoning control used or unavailable evidence only-for merge readiness",
        "reasoning control used or unavailable evidence if the reviewer can confirm it",
        "reasoning control used or unavailable evidence when the reviewer can confirm it",
    ] {
        let output = validate_sentinel_replacement(
            "reasoning control used or unavailable evidence",
            replacement,
        )?;
        assert!(!output.status.success(), "accepted {replacement:?}");
        assert!(stderr(&output).contains("reasoning-control evidence must be affirmative"));
    }
    Ok(())
}

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
    let output = validate_sentinel_replacement(
        "touched implementation-file LOC evidence when applicable",
        "touched implementation-file LOC evidence where applicable",
    )?;
    assert!(output.status.success(), "{}", stderr(&output));
    Ok(())
}

#[test]
fn validator_cli_rejects_non_affirmative_reasoning_control_paragraph() -> TestResult {
    for replacement in [
        "Reasoning control: the packaged Sentinel definition MUST run with the highest available reasoning setting, currently model_reasoning_effort = \"xhigh\" is optional. Reviewer evidence MUST record explicit unavailable evidence.\n\n",
        "No Reasoning control: the packaged Sentinel definition MUST run with the highest available reasoning setting, currently model_reasoning_effort = \"xhigh\". Reviewer evidence MUST record explicit unavailable evidence.\n\n",
        "Negated Reasoning control: the packaged Sentinel definition MUST run with the highest available reasoning setting, currently model_reasoning_effort = \"xhigh\". Reviewer evidence MUST record explicit unavailable evidence.\n\n",
        "Not Reasoning control: the packaged Sentinel definition MUST run with the highest available reasoning setting, currently model_reasoning_effort = \"xhigh\". Reviewer evidence MUST record explicit unavailable evidence.\n\n",
        "Reasoning control: no packaged Sentinel definition MUST run with the highest available reasoning setting, currently model_reasoning_effort = \"xhigh\". Reviewer evidence MUST record explicit unavailable evidence.\n\n",
        "Reasoning control: the packaged Sentinel definition MUST run with the highest available reasoning setting, currently model_reasoning_effort = \"xhigh\". Reviewer evidence MUST record explicit unavailable evidence is forbidden.\n\n",
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
        "Reasoning control: the packaged Sentinel definition MUST run with the highest available reasoning setting, currently model_reasoning_effort = \"xhigh\". If an invocation surface is available without reasoning controls, the reviewer evidence MUST record explicit unavailable evidence.\n\n",
        "Reasoning control: the packaged Sentinel definition MUST run with the highest available reasoning setting, currently model_reasoning_effort = \"xhigh\". If the invocation surface is missing reasoning controls, the reviewer evidence MUST record explicit unavailable evidence.\n\n",
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
