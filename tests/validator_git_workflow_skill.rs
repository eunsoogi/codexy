#[test]
fn git_workflow_validates_pr_suffix_without_issue_number() -> Result<(), Box<dyn std::error::Error>>
{
    let skill = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("plugins/codexy/skills/git-workflow/SKILL.md"),
    )?;

    assert!(!skill.contains(
        "if [ -n \"${issue_number:-}\" ]; then\n  if ! scripts/validate-plugin-config --check-merge-message --expected-issue \"$issue_number\" --expected-pr \"$pr_number\" --merge-message-file \"$merge_message_file\"; then"
    ));
    assert!(!skill.contains(
        "if [ -n \"$expected_issue_number\" ]; then\n  if ! scripts/validate-plugin-config --check-merge-message --expected-issue \"$expected_issue_number\" --expected-pr \"$pr_number\" --merge-message-file \"$commit_message_file\"; then"
    ));
    assert!(
        skill
            .contains("merge_validation_args=(--check-merge-message --expected-pr \"$pr_number\")")
    );
    assert!(skill.contains(
        "post_merge_validation_args=(--check-merge-message --expected-pr \"$pr_number\")"
    ));
    Ok(())
}
