use crate::support::FixtureCommand as Command;

#[test]
fn issue_title_hook_delegates_conventional_title_validation()
-> Result<(), Box<dyn std::error::Error>> {
    reject_issue_title("Fix(agents) reject negated sentinel evidence")
}

#[test]
fn issue_title_hook_rejects_lifecycle_event_invocation_without_model_context()
-> Result<(), Box<dyn std::error::Error>> {
    let issue_hook = Command::new(hook_script("codexy-issue-title-check.sh"))
        .arg("UserPromptSubmit")
        .output()?;
    assert!(
        !issue_hook.status.success(),
        "issue title hard check retained a lifecycle context mode\n{}",
        output_text(&issue_hook)
    );
    assert!(
        !output_text(&issue_hook).contains("hookSpecificOutput"),
        "issue title lifecycle invocation emitted model context: {}",
        output_text(&issue_hook),
    );
    Ok(())
}

fn reject_issue_title(title: &str) -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new(hook_script("codexy-issue-title-check.sh"))
        .args(["--issue-title", title])
        .output()?;
    assert!(
        !output.status.success(),
        "issue title hook should reject {title:?}"
    );
    assert!(
        output_text(&output).contains("issue title must not use Conventional Commit style"),
        "unexpected output: {}",
        output_text(&output)
    );
    Ok(())
}

fn hook_script(name: &str) -> std::path::PathBuf {
    codexy_runtime::paths::repository_root()
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
