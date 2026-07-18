use std::path::Path;

use super::copy_dir;

pub(crate) type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

pub(crate) fn assert_rejected(policy: &str, expected: &str) -> TestResult {
    assert_policy_rejected(duplicate_recipient_section(policy)?, expected)
}

pub(crate) fn assert_policy_rejected(skill: String, expected: &str) -> TestResult {
    let output = validate(skill)?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "routing bypass unexpectedly passed"
    );
    assert!(
        stderr.contains(expected),
        "routing rejection must name {expected}: {stderr}"
    );
    Ok(())
}

pub(crate) fn assert_accepted(skill: String) -> TestResult {
    let output = validate(skill)?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

pub(crate) fn duplicate_recipient_section(policy: &str) -> TestResult<String> {
    let skill = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("plugins/codexy/skills/codex-orchestration/SKILL.md"),
    )?;
    Ok(skill.replacen(
        "## Read Next",
        &format!("## Recipient Model Routing\n\n{policy}\n\n## Read Next"),
        1,
    ))
}

pub(crate) fn validate(skill: String) -> TestResult<std::process::Output> {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_dir(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy"),
        &plugin_root,
    )?;
    std::fs::write(
        plugin_root.join("skills/codex-orchestration/SKILL.md"),
        skill,
    )?;
    super::validator_routing(&plugin_root)
}
