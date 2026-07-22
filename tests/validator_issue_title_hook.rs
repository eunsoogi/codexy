use std::process::Command;

#[test]
fn issue_title_hook_rejects_prefix_only_conventional_title()
-> Result<(), Box<dyn std::error::Error>> {
    reject_issue_title("Fix(agents) reject negated sentinel evidence")
}

#[test]
fn issue_title_hook_rejects_newline_after_conventional_prefix()
-> Result<(), Box<dyn std::error::Error>> {
    reject_issue_title("Fix(agents)\nreject negated sentinel evidence")
}

#[test]
fn issue_title_hook_rejects_bare_colon_conventional_title() -> Result<(), Box<dyn std::error::Error>>
{
    reject_issue_title("Fix(agents):")?;
    reject_issue_title("Fix!: ")
}

#[test]
fn issue_title_hook_rejects_repeated_colon_conventional_title()
-> Result<(), Box<dyn std::error::Error>> {
    reject_issue_title("Fix(agents)::")?;
    reject_issue_title("Fix!::")
}

#[test]
fn issue_title_hook_rejects_adjacent_colon_conventional_title()
-> Result<(), Box<dyn std::error::Error>> {
    reject_issue_title("Fix(agents):reject")?;
    reject_issue_title("Fix!:break")?;
    reject_issue_title("Fix:break")
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
