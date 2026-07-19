use std::path::Path;

pub(crate) type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

pub(crate) fn assert_rejected(policy: &str, expected: &str) -> TestResult {
    assert_policy_rejected(duplicate_recipient_section(policy)?, expected)
}

pub(crate) fn assert_policy_rejected(skill: String, expected: &str) -> TestResult {
    let errors = validate(skill)?;
    assert!(!errors.is_empty(), "routing bypass unexpectedly passed");
    assert!(
        errors.iter().any(|error| error.contains(expected)),
        "routing rejection must name {expected}: {errors:#?}"
    );
    Ok(())
}

pub(crate) fn assert_accepted(skill: String) -> TestResult {
    let errors = validate(skill)?;
    assert!(errors.is_empty(), "{errors:#?}");
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

pub(crate) fn validate(skill: String) -> TestResult<Vec<String>> {
    let temp = tempfile::tempdir()?;
    let path = temp.path().join("SKILL.md");
    std::fs::write(&path, skill)?;
    Ok(codexy_runtime::validation::orchestration_routing_diagnostics(&path)?)
}
