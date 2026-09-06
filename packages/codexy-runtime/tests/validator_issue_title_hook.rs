use crate::support::FixtureCommand as Command;

#[test]
fn issue_title_hook_delegates_conventional_title_validation()
-> Result<(), Box<dyn std::error::Error>> {
    reject_issue_title("Fix(agents) reject negated sentinel evidence")
}

#[test]
fn issue_title_hook_preserves_descriptive_prose_and_rejects_label_prefixes()
-> Result<(), Box<dyn std::error::Error>> {
    for title in [
        "Reduce CI build time",
        "CI fails when cache restore times out",
        "Fix cache restoration after a runner restart",
        "Support HTTP/2 requests on port 8080",
        "Explain cache failures: retain the original error",
    ] {
        let output = Command::new(hook_script("codexy-issue-title-check.sh"))
            .args(["--issue-title", title])
            .output()?;
        assert!(
            output.status.success(),
            "issue title hook should accept {title:?}: {}",
            output_text(&output)
        );
    }
    for title in [
        "CI: reduce build time",
        "CI : reduce build time",
        "Fix (task) : reject invalid titles",
        "CI： reduce build time",
        "CI - reduce build time",
        "CI – reduce build time",
        "CI — reduce build time",
        "[CI] Reduce build time",
        "CI",
        "Fix",
        "lowercase prose",
        "Multiline\ntitle",
    ] {
        reject_issue_title(title)?;
    }
    Ok(())
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
        output_text(&output).contains("issue title must"),
        "unexpected output: {}",
        output_text(&output)
    );
    Ok(())
}

fn hook_script(name: &str) -> std::path::PathBuf {
    codexy_runtime::paths::repository_root()
        .join("plugins/codexy-github/hooks")
        .join(name)
}

fn output_text(output: &std::process::Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}
