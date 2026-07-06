use std::process::Command;

#[test]
fn issue_title_hook_accepts_uppercase_descriptive_title() -> Result<(), Box<dyn std::error::Error>>
{
    let output = issue_hook("Duplicate Codex review request guard")?;
    assert!(
        output.status.success(),
        "issue title hook should accept descriptive title\n{}",
        output_text(&output)
    );
    Ok(())
}

#[test]
fn issue_title_validator_accepts_uppercase_descriptive_title()
-> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--check-issue-title",
            "--issue-title",
            "Duplicate Codex review request guard",
        ])
        .output()?;
    assert!(
        output.status.success(),
        "issue title validator should accept descriptive title\n{}",
        output_text(&output)
    );
    Ok(())
}

#[test]
fn issue_title_hook_rejects_conventional_title_prefixes() -> Result<(), Box<dyn std::error::Error>>
{
    reject_issue_title("Fix(agents): reject negated sentinel evidence")?;
    reject_issue_title("fix(workflow): guard duplicate reviews")?;
    reject_issue_title("Docs: explain workflow")?;
    Ok(())
}

#[test]
fn issue_title_hook_rejects_lowercase_descriptive_title() -> Result<(), Box<dyn std::error::Error>>
{
    let output = issue_hook("duplicate Codex review request guard")?;
    assert!(
        !output.status.success(),
        "issue title hook should reject lowercase title"
    );
    assert!(
        output_text(&output).contains("issue title must start with uppercase descriptive prose"),
        "unexpected output: {}",
        output_text(&output)
    );
    Ok(())
}

#[test]
fn issue_title_context_includes_runnable_hard_checks() -> Result<(), Box<dyn std::error::Error>> {
    let issue_hook = Command::new(hook_script("codexy-issue-title-check.sh"))
        .arg("UserPromptSubmit")
        .output()?;
    assert!(
        issue_hook.status.success(),
        "issue title context hook should succeed\n{}",
        output_text(&issue_hook)
    );
    assert!(
        output_text(&issue_hook)
            .contains("scripts/validate-plugin-config --check-issue-title --issue-title"),
        "issue title context should include validator fallback: {}",
        output_text(&issue_hook)
    );

    Ok(())
}

fn reject_issue_title(title: &str) -> Result<(), Box<dyn std::error::Error>> {
    let output = issue_hook(title)?;
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

fn issue_hook(title: &str) -> Result<std::process::Output, Box<dyn std::error::Error>> {
    Ok(Command::new(hook_script("codexy-issue-title-check.sh"))
        .args(["--issue-title", title])
        .output()?)
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
