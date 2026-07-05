#[test]
fn git_workflow_does_not_accept_thumbs_up_only_codex_completion()
-> Result<(), Box<dyn std::error::Error>> {
    let skill = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("plugins/codexy/skills/git-workflow/SKILL.md"),
    )?;

    assert!(
        !skill.contains("thumbs-up reaction"),
        "aggregate thumbs-up reactions do not prove the actor was Codex"
    );
    assert!(
        !skill.contains("such as `+1`"),
        "Codex completion signals should require connector-authored output, not bare reactions"
    );
    Ok(())
}

#[test]
fn git_workflow_fetches_inline_review_comment_commit_oids() -> Result<(), Box<dyn std::error::Error>>
{
    let reference = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("plugins/codexy/skills/git-workflow/references/pr-review-and-handoff.md"),
    )?;

    assert!(
        reference.contains("commit { oid }"),
        "reviewThreads comment evidence must include inline review comment commit OIDs"
    );
    assert!(
        reference.contains("git status --short --branch > pr-state.worktreeStatus.txt")
            && reference.contains("worktreeStatus: $worktreeStatus"),
        "completion-handoff capture must include current local git status evidence"
    );
    Ok(())
}
