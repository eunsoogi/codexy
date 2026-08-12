use super::{TestResult, assert_case, plugin_root, repository};

#[test]
fn initial_repository_policy_survives_shell_directory_changes() -> TestResult {
    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let owned = repository(workspace.path(), "owned", "git@github.com:eunsoogi/codexy.git")?;
    let foreign = repository(workspace.path(), "foreign", "https://github.com/openai/codex.git")?;
    assert_case(&root, &owned, &format!("cd {} && gh issue create --repo eunsoogi/codexy --title invalid", foreign.display()), true, &[])?;
    std::fs::write(foreign.join("valid-issue.md"), "## Problem\nP\n## Scope\nS\n## Acceptance Criteria\nA\n## Verification\nV")?;
    assert_case(&root, &owned, &format!("cd {} && gh issue create --repo eunsoogi/codexy --title 'Valid issue' --body-file valid-issue.md", foreign.display()), false, &[])?;
    assert_case(&root, &owned, &format!("cd {} && gh issue create --repo openai/codex --title invalid", foreign.display()), false, &[])?;
    for command in [
        format!("sudo -D {} gh issue create --repo eunsoogi/codexy --title invalid", foreign.display()),
        format!("sudo --chdir={} gh issue create --repo eunsoogi/codexy --title invalid", foreign.display()),
        format!("cd {} && git push --force https://github.com/eunsoogi/codexy.git topic", foreign.display()),
        format!("sudo -D {} git push --force https://github.com/eunsoogi/codexy.git topic", foreign.display()),
    ] {
        assert_case(&root, &owned, &command, true, &[])?;
    }
    for command in [
        format!("cd {} && git push --force https://github.com/openai/codex.git topic", foreign.display()),
        format!("cd {} && gh issue create --repo eunsoogi/codexy --title invalid", owned.display()),
    ] {
        assert_case(&root, &foreign, &command, false, &[])?;
    }
    Ok(())
}

#[test]
fn valid_policy_with_a_foreign_remote_remains_target_discriminating() -> TestResult {
    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let foreign = repository(workspace.path(), "foreign", "https://github.com/openai/codex.git")?;
    let policy = foreign.join(".codex/repository-github-policy.json");
    std::fs::create_dir_all(policy.parent().ok_or("policy parent")?)?;
    std::fs::write(policy, "{\"schema\":\"codexy.repository-github-policy/v1\",\"repository\":\"eunsoogi/codexy\"}")?;
    assert_case(&root, &foreign, "gh issue create --repo openai/codex --title invalid", false, &[])?;
    assert_case(&root, &foreign, "gh issue create --repo eunsoogi/codexy --title invalid", true, &[])
}

#[test]
fn invalid_project_policy_fails_closed_for_repository_mutations() -> TestResult {
    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let owned = repository(workspace.path(), "owned", "git@github.com:eunsoogi/codexy.git")?;
    std::fs::write(
        owned.join(".codex/repository-github-policy.json"),
        "{\"schema\":\"codexy.repository-github-policy/v1\",\"schema\":\"duplicate\",\"repository\":\"eunsoogi/codexy\"}",
    )?;
    assert_case(&root, &owned, "gh issue create --title invalid", true, &[])
}

#[cfg(unix)]
#[test]
fn symlinked_project_policy_fails_closed_for_repository_mutations() -> TestResult {
    use std::os::unix::fs::symlink;

    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let owned = repository(workspace.path(), "owned", "git@github.com:eunsoogi/codexy.git")?;
    let policy = owned.join(".codex/repository-github-policy.json");
    let replacement = workspace.path().join("policy.json");
    std::fs::write(&replacement, std::fs::read(&policy)?)?;
    std::fs::remove_file(&policy)?;
    symlink(replacement, policy)?;
    assert_case(&root, &owned, "gh issue create --title invalid", true, &[])
}
