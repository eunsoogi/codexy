type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn git_workflow_routes_local_git_rules_to_reference() -> TestResult {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let skill = std::fs::read_to_string(root.join("plugins/codexy/skills/git-workflow/SKILL.md"))?;
    let reference = std::fs::read_to_string(
        root.join("plugins/codexy/skills/git-workflow/references/local-git-and-branches.md"),
    )?;

    assert!(skill.contains("references/local-git-and-branches.md"));
    assert!(skill.contains("MUST read `references/local-git-and-branches.md`"));
    assert!(reference.contains("git worktree add -b codexy/<issue-or-scope>"));
    assert!(reference.contains("MUST NOT force-push task branches"));
    assert!(reference.contains("MUST use Conventional Commit style"));
    assert!(reference.contains("MUST preserve both sides' intended"));
    Ok(())
}
