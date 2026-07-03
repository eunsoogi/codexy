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
