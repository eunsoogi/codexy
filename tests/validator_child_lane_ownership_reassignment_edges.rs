
fn run_validator(evidence: &str) -> Result<std::process::Output, Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let evidence_path = temp.path().join("handoff.md");
    std::fs::write(&evidence_path, evidence)?;

    crate::support::validator_child_lane_ownership_file(&evidence_path)
}

#[test]
fn validator_rejects_not_yet_granted_reassignment() -> Result<(), Box<dyn std::error::Error>> {
    let output = run_validator(
        "Lane ownership: child-owned\n\
         Review response: parent-authored implementation commit abc123 fixed feedback.\n\
         Maintainer reassignment: explicit maintainer reassignment to parent not yet granted\n",
    )?;

    assert!(
        !output.status.success(),
        "validator should reject not-yet-granted reassignment evidence"
    );
    Ok(())
}

#[test]
fn validator_allows_reassignment_notes_key() -> Result<(), Box<dyn std::error::Error>> {
    let output = run_validator(
        "Lane ownership: child-owned\n\
         Review response: parent-authored implementation commit abc123 fixed feedback.\n\
         Maintainer reassignment notes: explicit maintainer reassignment to parent\n",
    )?;

    assert!(
        output.status.success(),
        "validator should accept reassignment notes as affirmative evidence\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_normalizes_list_prefixes_for_external_review_response_metadata()
-> Result<(), Box<dyn std::error::Error>> {
    let table = "| Task classification | Decision |\n| --- | --- |\n| Lane type | review response |\n| Secondary surfaces | validators |\n| Owner decision | external/human-owned implementation lane |\n| Atomic scope | issue-sized |\n| Required skills | task-classification |\n| Required tools/evidence | validator |\n| First allowed action | wait |\n| Stop/blocker | None |\n";
    for prefix in ["", "1. ", "+ ", "- [ ] "] {
        let output = run_validator(&format!("{table}\n{prefix}Review response: child-authored commit def456 fixed feedback\n"))?;
        assert!(!output.status.success(), "validator should reject `{prefix}` child-authored external review response");
    }
    assert!(run_validator(&format!("{table}\nReview response: human-authored commit def456 fixed feedback\n"))?.status.success());
    Ok(())
}

#[test]
fn validator_separates_later_parent_fix_after_both_pr_metadata_aliases()
-> Result<(), Box<dyn std::error::Error>> {
    let table = "| Task classification | Decision |\n| --- | --- |\n| Lane type | implementation |\n| Secondary surfaces | workflow |\n| Owner decision | current-thread-owned child implementation lane |\n| Atomic scope | issue-sized |\n| Required skills | task-classification |\n| Required tools/evidence | goal |\n| First allowed action | create branch |\n| Stop/blocker | None |\n";
    for boundary in ["PR", "Pull request"] {
        let output = run_validator(&format!(
            "{table}\nPR: #468\nReview response: child-authored commit def456 fixed feedback\n{boundary}: #999\nReview response: parent-authored implementation commit abc123 fixed feedback\n"
        ))?;
        assert!(output.status.success(), "{boundary} should end the child lane");
    }
    Ok(())
}
