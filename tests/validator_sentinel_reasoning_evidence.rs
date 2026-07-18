use std::path::Path;
use std::process::{Command, Output};

use crate::support;

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
        "Every approval MUST decide whether to reference reasoning control used or unavailable evidence",
        "Every approval MUST reference the current diff or head, lane scope, but not reasoning control used or unavailable evidence",
        "Every approval MUST reference the current diff or head, lane scope, but is required not to reference reasoning control used or unavailable evidence",
        "Every approval MUST reference the current diff or head, lane scope, but is required to not reference reasoning control used or unavailable evidence",
        "Every approval MUST inspect reasoning control used or unavailable evidence",
        "Every approval MUST omit reasoning control used or unavailable evidence",
        "Every approval MUST skip reasoning control used or unavailable evidence",
        "Every approval MUST NOT omit reasoning control used or unavailable evidence if applicable",
        "reasoning control used or unavailable evidence does not have to be supplied",
        "reasoning control used or unavailable evidence does not\nneed to be supplied",
        "reasoning control used or unavailable evidence needn't be supplied",
        "reasoning control used or unavailable evidence isn't required",
        "reasoning control used or unavailable evidence isn't necessary",
        "reasoning control used or unavailable evidence is not explicitly required",
        "reasoning control used or unavailable evidence isn't explicitly required",
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
        "reasoning control used or unavailable evidence whenever possible",
        "reasoning control used or unavailable evidence is required only when tools expose it",
        "reasoning control used or unavailable evidence is required to not be included",
        "reasoning control used or unavailable evidence isn't a requirement",
        "except reasoning control used or unavailable evidence",
        "reasoning control used or unavailable evidence only when tools expose it",
        "reasoning control used or unavailable evidence. The reasoning-control evidence can be skipped",
        "reasoning control used or unavailable evidence, although optional",
        "reasoning control used or unavailable evidence, optional",
    ] {
        let output = validate_sentinel_replacement(
            "reasoning control used or unavailable evidence",
            replacement,
        )?;
        assert!(!output.status.success(), "accepted {replacement:?}");
        assert!(stderr(&output).contains("reasoning-control evidence must be affirmative"));
    }
    let needle = "reasoning control used or unavailable evidence, direct reviewer passes performed";
    let output = validate_sentinel_replacement(
        needle,
        "reasoning control used or unavailable evidence, but reviewers may omit reasoning-control evidence, direct reviewer passes performed",
    )?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("reasoning-control evidence must be affirmative"));
    let output = validate_sentinel_edit(|sentinel| {
        Ok(sentinel.replacen(
            "\n\"\"\"\n",
            "\nDo not record reasoning control used or unavailable evidence.\n\"\"\"\n",
            1,
        ))
    })?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("reasoning-control evidence must be affirmative"));
    let output = validate_sentinel_replacement(
        "reasoning control used or unavailable evidence, direct reviewer passes performed",
        "reasoning control used or unavailable evidence, direct reviewer passes performed",
    )?;
    assert!(output.status.success(), "{}", stderr(&output));
    let output = validate_sentinel_edit(|sentinel| {
        Ok(sentinel.replace(
            "and any unresolved risk. MUST identify formatting-only LOC remediation before approving readiness.",
            "and any unresolved risk. Every approval MUST NOT omit reasoning control used or unavailable evidence, but not direct reviewer passes performed.. MUST identify formatting-only LOC remediation before approving readiness.",
        ))
    })?;
    assert!(
        !output.status.success(),
        "accepted mixed-polarity approval evidence"
    );
    assert!(stderr(&output).contains("reviewer gate contract is missing"));
    Ok(())
}
fn validate_sentinel_replacement(needle: &str, replacement: &str) -> TestResult<Output> {
    validate_sentinel_edit(|sentinel| Ok(sentinel.replace(needle, replacement)))
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
