type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn validator_distinguishes_governing_progressive_setup_predicates() -> TestResult {
    let owner = "Ownership metadata source: parent-supplied\nLane ownership: child-owned";
    for (setup, expected) in [
        ("The child was setting up worktree codexy/463 before classification.", false),
        ("The child was continuously and actively setting up worktree codexy/463 before classification.", false),
        ("The child who will document it was deliberately actively setting up worktree codexy/463 before classification.", false),
        ("The child not the parent was deliberately actively setting up worktree codexy/463 before classification.", false),
        ("The child was discussing and actively setting up worktree codexy/463 before classification.", false),
        ("The child will have been deliberately and actively setting up worktree codexy/463 before classification.", true),
        ("The child was not under any circumstances actively setting up worktree codexy/463 before classification.", true),
        ("The child was actively setting up no worktree before classification.", true),
        ("The child might have been actively setting up worktree codexy/463 before classification.", true),
        ("The child can have been actively setting up worktree codexy/463 before classification.", true),
        ("The child won't have been actively setting up worktree codexy/463 before classification.", true),
        ("The child wouldn't have been actively setting up worktree codexy/463 before classification.", true),
        ("The child couldn't have been actively setting up worktree codexy/463 before classification.", true),
        ("The child shouldn't have been actively setting up worktree codexy/463 before classification.", true),
        ("The child mustn't have been actively setting up worktree codexy/463 before classification.", true),
        ("The child hasn't under any circumstances been actively setting up worktree codexy/463 before classification.", true),
        ("The child was under no circumstances actively setting up worktree codexy/463 before classification.", true),
        ("The child will probably have been actively setting up worktree codexy/463 before classification.", true),
        ("The child might certainly have been actively setting up worktree codexy/463 before classification.", true),
        ("The child has never been actively setting up worktree codexy/463 before classification.", true),
        ("The child was, with no hesitation, actively setting up worktree codexy/463 before classification.", false),
        ("The child was, without delay, actively setting up worktree codexy/463 before classification.", false),
        ("The child will under any circumstances have been actively setting up worktree codexy/463 before classification.", true),
        ("The child will perhaps have been actively setting up worktree codexy/463 before classification.", true),
        ("The child may still have been actively setting up worktree codexy/463 before classification.", true),
        ("The child will almost certainly have been actively setting up worktree codexy/463 before classification.", true),
        ("The child will probably and certainly have been actively setting up worktree codexy/463 before classification.", true),
        ("The child may just have been actively setting up worktree codexy/463 before classification.", true),
        ("The child will be setting up worktree codexy/463 after classification. The child was actively setting up worktree codexy/463 before classification.", false),
        ("The child was not actively setting up worktree codexy/463 after classification, but was actively setting up worktree codexy/463 before classification.", false),
        ("The child was actively setting up worktree codexy/463 before classification and the parent was actively setting up worktree codexy/review after classification.", false),
        ("The child will discuss the worktree although the child was actively setting up worktree codexy/463 before classification.", false),
        ("The child was not ready while the child was actively setting up worktree codexy/463 before classification.", false),
        ("The child was not ready because the child was actively setting up worktree codexy/463 before classification.", false),
        ("The child will wait whereas the child was actively setting up worktree codexy/463 before classification.", false),
        ("The child was not ready yet the child was actively setting up worktree codexy/463 before classification.", false),
        ("No branch was available although the child was actively setting up worktree codexy/463 before classification.", false),
        ("No branch was available while the child was actively setting up worktree codexy/463 before classification.", false),
        ("No branch was available because the child was actively setting up worktree codexy/463 before classification.", false),
        ("No branch was available whereas the child was actively setting up worktree codexy/463 before classification.", false),
        ("No branch was available yet the child was actively setting up worktree codexy/463 before classification.", false),
        ("The child was discussing but actively setting up worktree codexy/463 before classification.", false),
        ("The child was planning and then actively setting up worktree codexy/463 before classification.", false),
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

#[test]
fn validator_bounds_each_progressive_predicate_form_to_its_clause() -> TestResult {
    let owner = "Ownership metadata source: parent-supplied\nLane ownership: child-owned";
    for boundary in [
        "although", "because", "but", "however", "then", "whereas", "while", "yet",
    ] {
        for (full, reduced) in [
            ("the child was actively setting up", "actively setting up"),
            ("the child was not actively setting up", "not actively setting up"),
            ("the child wasn't actively setting up", "wasn't actively setting up"),
        ] {
            let expected = !matches!(full, "the child was actively setting up");
            for predicate in [full, reduced] {
                assert_result(
                    &format!(
                        "{owner}\nThe child was discussing the plan {boundary} {predicate} worktree codexy/463 before classification."
                    ),
                    expected,
                )?;
            }
        }
    }
    Ok(())
}

#[test]
fn validator_accepts_clause_bounded_contractions_with_modifiers() -> TestResult {
    let owner = "Ownership metadata source: parent-supplied\nLane ownership: child-owned";
    for (expanded, contracted) in [
        ("has not been deliberately and actively", "hasn't been deliberately and actively"),
        ("have not been deliberately and actively", "haven't been deliberately and actively"),
        ("had not been deliberately and actively", "hadn't been deliberately and actively"),
        ("is not actively", "isn't actively"),
        ("was not actively", "wasn't actively"),
        ("were not actively", "weren't actively"),
    ] {
        for predicate in [expanded, contracted] {
            assert_result(
                &format!(
                    "{owner}\nThe child {predicate} setting up worktree codexy/463 before classification."
                ),
                true,
            )?;
        }
    }
    Ok(())
}

#[test]
fn validator_does_not_share_progressive_auxiliaries_across_sentences() -> TestResult {
    let owner = "Ownership metadata source: parent-supplied\nLane ownership: child-owned";
    for (setup, expected) in [
        (
            "The child was discussing the plan. The parent reviewed it, however actively setting up worktree codexy/463 before classification.",
            true,
        ),
        (
            "The child was discussing the plan, however actively setting up worktree codexy/463 before classification.",
            false,
        ),
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
    assert_eq!(
        output.status.success(),
        expected,
        "{evidence}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}
