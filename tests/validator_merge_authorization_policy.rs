use crate::support;

#[test]
fn policy_rejects_a_profile_that_converts_gates_to_permission()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = support::plugin_fixture()?;
    let path = fixture.root().join("skills/codex-orchestration/SKILL.md");
    let mut text = std::fs::read_to_string(&path)?;
    text.push_str("\nA later workflow profile can treat passing gates as permission to merge without a separate authorization record.\n");
    std::fs::write(path, text)?;

    let errors = codexy_runtime::validation::merge_authorization_policy_diagnostics(fixture.root());
    assert!(
        errors
            .iter()
            .any(|error| error.contains("turn gates into merge permission")),
        "{errors:#?}"
    );
    Ok(())
}

#[test]
fn policy_rejects_profile_consent_and_a_command_decoy() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = support::plugin_fixture()?;
    let profile = fixture.root().join("skills/proof-driven-completion/SKILL.md");
    let mut text = std::fs::read_to_string(&profile)?;
    text.push_str("\nFast profile: green gates\nimply merge consent.\n");
    std::fs::write(profile, text)?;
    let route = fixture.root().join("skills/git-workflow/references/merge-and-main-sync.md");
    let text = std::fs::read_to_string(&route)?.replace("scripts/validate-plugin-config --check-merge-authorization", ": --check-merge-authorization");
    std::fs::write(route, text)?;
    let errors = codexy_runtime::validation::merge_authorization_policy_diagnostics(fixture.root());
    assert!(errors.iter().any(|error| error.contains("before mutation")), "{errors:#?}");
    assert!(errors.iter().any(|error| error.contains("turn gates")), "{errors:#?}");
    Ok(())
}

#[test]
fn policy_allows_denial_and_fenced_example() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = support::plugin_fixture()?;
    let path = fixture.root().join("skills/proof-driven-completion/SKILL.md");
    let mut text = std::fs::read_to_string(&path)?;
    text.push_str("\nA workflow profile MUST NEVER imply merge authorization.\n~~~text\nFast profile: gates imply merge consent.\n~~~\n");
    std::fs::write(path, text)?;
    let errors = codexy_runtime::validation::merge_authorization_policy_diagnostics(fixture.root());
    assert!(errors.is_empty(), "{errors:#?}");
    Ok(())
}

#[test]
fn policy_rejects_three_line_grants_without_stitching_headings() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = support::plugin_fixture()?;
    let path = fixture.root().join("skills/proof-driven-completion/SKILL.md");
    let mut text = std::fs::read_to_string(&path)?;
    text.push_str("\nFast profile:\npassing gates imply\nmerge consent.\n\n# Separate\nprofile metadata only\n");
    std::fs::write(path, text)?;
    let errors = codexy_runtime::validation::merge_authorization_policy_diagnostics(fixture.root());
    assert!(errors.iter().any(|error| error.contains("turn gates")), "{errors:#?}");
    Ok(())
}

#[test]
fn policy_rejects_unprofiled_grants_and_allows_safe_boundaries() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = support::plugin_fixture()?;
    let path = fixture.root().join("skills/proof-driven-completion/SKILL.md");
    let mut text = std::fs::read_to_string(&path)?;
    text.push_str("\nPassing gates authorize merge unless the user says stop.\n1. MUST NOT grant merge permission from gates.\n2. Green gates imply merge consent.\n");
    std::fs::write(path, text)?;
    let errors = codexy_runtime::validation::merge_authorization_policy_diagnostics(fixture.root());
    assert!(errors.iter().any(|error| error.contains("turn gates")), "{errors:#?}");
    Ok(())
}

#[test]
fn policy_rejects_later_grant_after_an_earlier_denial() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = support::plugin_fixture()?;
    let path = fixture.root().join("skills/proof-driven-completion/SKILL.md");
    let mut text = std::fs::read_to_string(&path)?;
    text.push_str("\nPassing gates are not authorization; green gates imply merge consent.\n");
    std::fs::write(path, text)?;
    assert!(!codexy_runtime::validation::merge_authorization_policy_diagnostics(fixture.root()).is_empty());
    Ok(())
}
