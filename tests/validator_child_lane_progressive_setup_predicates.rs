type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn validator_distinguishes_governing_progressive_setup_predicates() -> TestResult {
    let owner = "Ownership metadata source: parent-supplied\nLane ownership: child-owned";
    for (setup, expected) in [
        ("The child was setting up worktree codexy/463 before classification.", false),
        ("The child was continuously and actively setting up worktree codexy/463 before classification.", false),
        ("The child was discussing and actively setting up worktree codexy/463 before classification.", false),
        ("The child was discussing setting up worktree codexy/463 after classification.", true),
        ("The child was considering setting up worktree codexy/463 after classification.", true),
        ("The child was planning on setting up worktree codexy/463 after classification.", true),
        ("The child wasn't setting up worktree codexy/463 before classification.", true),
        ("The child will be setting up worktree codexy/463 after classification.", true),
    ] {
        assert_result(&format!("{owner}\n{setup}"), expected)?;
    }
    Ok(())
}

fn assert_result(evidence: &str, expected: bool) -> TestResult {
    let temp = tempfile::tempdir()?;
    let path = temp.path().join("handoff.md");
    std::fs::write(&path, evidence)?;
    let output = crate::support::validator_child_lane_ownership_file(&path)?;
    assert_eq!(output.status.success(), expected, "{}", String::from_utf8_lossy(&output.stderr));
    Ok(())
}
