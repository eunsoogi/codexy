use std::process::Command;

#[allow(unused)]
mod support;

#[test]
fn validator_cli_requires_separate_pr_title_hook() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_plugin(&plugin_root)?;
    remove_user_prompt_hook_containing(&plugin_root, "codexy-pr-title-check.sh")?;

    let output = validate_hooks(&plugin_root)?;
    assert!(
        !output.status.success(),
        "validator should reject topology missing the hard PR title hook"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("UserPromptSubmit hook command must run hooks/codexy-pr-title-check.sh"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_cli_requires_separate_pr_label_hook() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_plugin(&plugin_root)?;
    remove_user_prompt_hook_containing(&plugin_root, "codexy-pr-label-check.sh")?;

    let output = validate_hooks(&plugin_root)?;
    assert!(
        !output.status.success(),
        "validator should reject topology missing the hard PR label hook"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("UserPromptSubmit hook command must run hooks/codexy-pr-label-check.sh"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_cli_requires_separate_merge_message_hook() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_plugin(&plugin_root)?;
    remove_user_prompt_hook_containing(&plugin_root, "codexy-merge-message-check.sh")?;

    let output = validate_hooks(&plugin_root)?;
    assert!(
        !output.status.success(),
        "validator should reject topology missing the hard merge-message hook"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("UserPromptSubmit hook command must run hooks/codexy-merge-message-check.sh"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn separated_hard_hooks_reject_issue_219_examples() -> Result<(), Box<dyn std::error::Error>> {
    let title = hook_script("codexy-pr-title-check.sh");
    let bad_title = Command::new(&title)
        .args(["--pr-title", "Require descriptive child thread titles"])
        .output()?;
    assert!(
        !bad_title.status.success(),
        "PR title hook should reject non-Conventional titles"
    );
    assert!(
        output_text(&bad_title).contains("PR title must use Conventional Commit style"),
        "unexpected output: {}",
        output_text(&bad_title)
    );

    let merge = hook_script("codexy-merge-message-check.sh");
    let bad_merge = Command::new(&merge)
        .args([
            "--expected-pr",
            "203",
            "--merge-message",
            "Refactor oversized Codexy skill instructions (#203)\n\nFixes #219\n",
        ])
        .output()?;
    assert!(
        !bad_merge.status.success(),
        "merge-message hook should reject non-Conventional squash subjects"
    );
    assert!(
        output_text(&bad_merge).contains("merge commit subject must use Conventional Commit style"),
        "unexpected output: {}",
        output_text(&bad_merge)
    );

    let temp = tempfile::tempdir()?;
    let pr_state = temp.path().join("unlabeled.json");
    std::fs::write(
        &pr_state,
        r#"{"number":219,"state":"OPEN","repository":"eunsoogi/codexy","labels":[],"repositoryLabels":[{"name":"type/fix"}]}"#,
    )?;
    let labels = hook_script("codexy-pr-label-check.sh");
    let bad_labels = Command::new(&labels)
        .args(["--pr-state-file", pr_state.to_str().ok_or("pr state")?])
        .output()?;
    assert!(
        !bad_labels.status.success(),
        "PR label hook should reject unlabeled PRs when repository labels exist"
    );
    assert!(
        output_text(&bad_labels).contains("PR labels missing label application evidence"),
        "unexpected output: {}",
        output_text(&bad_labels)
    );

    Ok(())
}

#[test]
fn validator_cli_rejects_unsafe_delegated_hard_hook_helper()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_plugin(&plugin_root)?;
    std::fs::write(
        plugin_root.join("hooks/codexy-readiness-guard.sh"),
        "#!/bin/sh\ngit status\n",
    )?;

    let output = validate_hooks(&plugin_root)?;
    assert!(
        !output.status.success(),
        "validator should reject unsafe delegated hard hook helpers"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("must not run \"git \""),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

fn validate_hooks(
    plugin_root: &std::path::Path,
) -> Result<std::process::Output, Box<dyn std::error::Error>> {
    Ok(Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--plugin-root",
            plugin_root.to_str().ok_or("plugin root path")?,
            "--check-hooks",
        ])
        .output()?)
}

fn copy_plugin(plugin_root: &std::path::Path) -> std::io::Result<()> {
    support::copy_dir(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy"),
        plugin_root,
    )
}

fn hook_script(name: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("plugins/codexy/hooks")
        .join(name)
}

fn output_text(output: &std::process::Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn remove_user_prompt_hook_containing(
    plugin_root: &std::path::Path,
    command_fragment: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let hooks_path = plugin_root.join("hooks/hooks.json");
    let mut hooks_config: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&hooks_path)?)?;
    let groups = hooks_config["hooks"]["UserPromptSubmit"]
        .as_array_mut()
        .ok_or("UserPromptSubmit hooks must be an array")?;
    groups.retain(|group| {
        let Some(hooks) = group["hooks"].as_array() else {
            return true;
        };
        !hooks.iter().any(|hook| {
            hook["command"]
                .as_str()
                .is_some_and(|command| command.contains(command_fragment))
        })
    });
    std::fs::write(&hooks_path, serde_json::to_string_pretty(&hooks_config)?)?;
    Ok(())
}
