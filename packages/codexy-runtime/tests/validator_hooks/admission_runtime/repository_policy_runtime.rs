use super::{TestResult, assert_case, assert_event_case, executable, plugin_root, repository};
use serde_json::json;

#[test]
fn initial_repository_policy_survives_shell_directory_changes() -> TestResult {
    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let owned = repository(workspace.path(), "owned", "git@github.com:eunsoogi/codexy.git")?;
    let foreign = repository(workspace.path(), "foreign", "https://github.com/openai/codex.git")?;
    assert_case(&root, &owned, &format!("cd {} && gh issue create --repo eunsoogi/codexy --title invalid", foreign.display()), true, &[])?;
    std::fs::write(foreign.join("valid-issue.md"), "## Problem\nP\n## Scope\nS\n## Acceptance Criteria\nA\n## Verification\nV")?;
    assert_case(&root, &owned, &format!("cd {} && gh issue create --repo eunsoogi/codexy --title 'Valid issue' --body-file valid-issue.md", foreign.display()), false, &[])?;
    assert_case(&root, &owned, &format!("cd {} && gh issue create --repo openai/codex --title invalid", foreign.display()), true, &[])?;
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
    ] {
        assert_case(&root, &foreign, &command, false, &[])?;
    }
    assert_case(&root, &foreign, &format!("cd {} && gh issue create --repo eunsoogi/codexy --title invalid", owned.display()), true, &[])?;
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
    assert_case(&root, &foreign, "gh issue create --repo openai/codex --title invalid", true, &[])?;
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

#[test]
fn ordinary_local_execution_is_admitted_without_a_protected_effect() -> TestResult {
    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let owned = repository(workspace.path(), "owned", "git@github.com:eunsoogi/codexy.git")?;
    for command in [
        "uv build",
        "cargo test --no-run",
        "python3 -m unittest",
        "pwsh -File ./scripts/check.ps1",
        "cd packages/getcodexy && uv build",
        "printf safe | sed 's/safe/safe/' > output.txt",
        "(cd packages/getcodexy && uv build)",
        "for file in README.md LICENSE; do sed -n '1p' \"$file\"; done",
    ] {
        assert_case(&root, &owned, command, false, &[])?;
    }
    for command in [
        "gh issue create --title invalid",
        "git push --force origin topic",
        "rm -rf /",
        "for file in README.md; do gh issue create --title invalid; done",
        "for file in README.md; do git push --force origin topic; done",
        "for file in README.md; do eval 'gh issue create --title invalid'; done",
        "eval '$COMMAND'",
    ] {
        assert_case(&root, &owned, command, true, &[])?;
    }
    Ok(())
}

#[test]
fn evaluated_sources_and_blank_lines_follow_effects_for_both_events() -> TestResult {
    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let owned = repository(workspace.path(), "owned", "git@github.com:eunsoogi/codexy.git")?;
    let printf = executable("printf")?;
    for event in ["PermissionRequest", "PreToolUse"] {
        assert_event_case(&root, event, &owned, "git diff --stat HEAD\n\ngit diff --stat", false, &[])?;
        assert_event_case(&root, event, &owned, "sh -c 'gh issue create --title invalid'", true, &[])?;
        assert_event_case(&root, event, &owned, "sh -c '$COMMAND'", true, &[])?;
        assert_event_case(&root, event, &owned, "if true; then gh repo view; fi", false, &[])?;
        assert_event_case(&root, event, &owned, "printf \"$(git status)\"", false, &[])?;
        assert_event_case(&root, event, &owned, "rm -rf \"$UNKNOWN_RUNTIME_VALUE\"", true, &[])?;
        assert_event_case(&root, event, &owned, &format!("'{}' \"$UNKNOWN_RUNTIME_VALUE\"", printf.display()), false, &[])?;
        assert_event_case(&root, event, &owned, "printf '%s\n' '$(gh issue create --title data)'", false, &[])?;
        assert_event_case(&root, event, &owned, "printf \"$(gh issue create --title invalid)\"", true, &[])?;
        assert_event_case(&root, event, &owned, "(printf safe) && { printf safe; }", false, &[])?;
        assert_event_case(&root, event, &owned, "python3 -c 'print(\"safe\")'", false, &[])?;
        assert_event_case(&root, event, &owned, "python3 -c 'print(\"gh issue create; rm -rf /\")'", false, &[])?;
        assert_event_case(&root, event, &owned, "node -e 'console.log(\"safe\")'", false, &[])?;
        assert_event_case(&root, event, &owned, "pwsh -Command 'Write-Output safe'", false, &[])?;
    }
    Ok(())
}

#[test]
fn dynamic_alias_operands_do_not_deny_without_a_following_alias_target() -> TestResult {
    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let owned = repository(workspace.path(), "owned", "git@github.com:eunsoogi/codexy.git")?;
    for event in ["PermissionRequest", "PreToolUse"] {
        assert_event_case(&root, event, &owned, "ln -s \"$UNKNOWN_RUNTIME_VALUE\" safe", false, &[])?;
        assert_event_case(&root, event, &owned, "ln -sf \"$UNKNOWN_RUNTIME_VALUE\" safe", true, &[])?;
    }
    Ok(())
}

#[test]
fn credential_operations_use_resolved_identity_for_both_events() -> TestResult {
    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let owned = repository(workspace.path(), "owned", "git@github.com:eunsoogi/codexy.git")?;
    let gh = executable("gh")?;
    for event in ["PermissionRequest", "PreToolUse"] {
        assert_event_case(&root, event, &owned, "printf '%s\n' 'gh auth token'", false, &[])?;
        assert_event_case(&root, event, &owned, "printf safe # gh auth token", false, &[])?;
        assert_event_case(&root, event, &owned, "printf '%s\n' 'eval $COMMAND'", false, &[])?;
        assert_event_case(&root, event, &owned, "gh auth token", true, &[])?;
        assert_event_case(&root, event, &owned, "GH_TOKEN=fixture gh issue list", true, &[])?;
        assert_event_case(&root, event, &owned, "gh api -H 'Authorization: Bearer fixture'", true, &[])?;
        assert_event_case(&root, event, &owned, "eval 'gh auth token'", true, &[])?;
        assert_event_case(&root, event, &owned, &format!("'{}' auth token", gh.display()), true, &[])?;
        assert_event_case(&root, event, &owned, "sh -c 'gh auth token'", true, &[])?;
        assert_event_case(&root, event, &owned, &format!("sh -c \"'{}' auth token\"", gh.display()), true, &[])?;
        assert_event_case(&root, event, &owned, "sh -c 'printf gh auth token'", false, &[])?;
        assert_event_case(&root, event, &owned, "eval 'printf gh auth token'", false, &[])?;
    }
    Ok(())
}

#[test]
fn protected_effect_denials_report_a_stable_effect_class() -> TestResult {
    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let owned = repository(workspace.path(), "owned", "git@github.com:eunsoogi/codexy.git")?;
    for (launcher, command, effect) in [
        ("codexy-repository-github-command", "gh issue create --title invalid", "REMOTE_MUTATION"),
        ("codexy-repository-github-command", "gh auth token", "CREDENTIAL_EXPOSURE"),
        ("codexy-destructive-command", "rm -rf /", "DESTRUCTIVE_EFFECT"),
        ("codexy-repository-github-command", "eval '$COMMAND'", "UNRESOLVED_PROTECTED_EFFECT"),
        ("codexy-destructive-command", "PATH=\"$UNKNOWN_RUNTIME_VALUE\" printf safe", "UNRESOLVED_PROTECTED_EFFECT"),
    ] {
        let input = json!({
            "hook_event_name": "PreToolUse",
            "tool_name": "Bash",
            "tool_input": {"command": command},
            "cwd": owned,
        });
        let output = super::concern_launchers::run_launcher(
            &root, launcher, "PreToolUse", &input, &[],
        )?;
        let denial: serde_json::Value = serde_json::from_slice(&output)?;
        let reason = denial["hookSpecificOutput"]["permissionDecisionReason"]
            .as_str()
            .ok_or("denial reason")?;
        assert!(reason.contains(effect), "{reason}");
    }
    Ok(())
}

#[test]
fn alias_carried_credential_operations_keep_their_typed_effect() -> TestResult {
    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let owned = repository(workspace.path(), "owned", "git@github.com:eunsoogi/codexy.git")?;
    let gh = executable("gh")?;
    for event in ["PermissionRequest", "PreToolUse"] {
        for command in [
            "gh auth token".to_owned(),
            format!("ln -sf '{}' safe && ./safe auth token", gh.display()),
        ] {
            let input = json!({
                "hook_event_name": event,
                "tool_name": "Bash",
                "tool_input": {"command": command},
                "cwd": owned,
            });
            let output = super::concern_launchers::run_launcher(
                &root, "codexy-repository-github-command", event, &input, &[],
            )?;
            let denial: serde_json::Value = serde_json::from_slice(&output)?;
            let reason = if event == "PermissionRequest" {
                &denial["hookSpecificOutput"]["decision"]["message"]
            } else {
                &denial["hookSpecificOutput"]["permissionDecisionReason"]
            };
            assert!(reason.as_str().unwrap_or_default().contains("CREDENTIAL_EXPOSURE"));
        }
    }
    Ok(())
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
