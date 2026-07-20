use std::process::Output;

fn run_ownership_validator(evidence: &str) -> Result<Output, Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let evidence_path = temp.path().join("handoff.md");
    std::fs::write(&evidence_path, evidence)?;

    crate::support::validator_child_lane_ownership_file(&evidence_path)
}

#[test]
fn validator_allows_keyed_absent_parent_reads_in_setup_bullet()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Lane ownership: child-owned
Implementation-surface reads:
- Child reads: src/validation/hooks.rs
- Parent reads: none
Review response: child-authored commit def456 fixed feedback
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should preserve keyed absent parent-read setup evidence\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_malformed_handoff_and_unseparated_pre_pr_setup() -> Result<(), Box<dyn std::error::Error>> {
    let table = "| Task classification | Decision |\n| --- | --- |\n| Lane type | implementation |\n| Secondary surfaces | workflow |\n| Owner decision | current-thread-owned child implementation lane |\n| Atomic scope | issue-sized |\n| Required skills | task-classification |\n| Required tools/evidence | goal |\n| First allowed action | create branch |\n";
    let malformed = ["Child branch codexy/461-table was created after classification.", "Review response: parent-authored implementation commit abc fixed feedback", "Source thread id: parent-461\nGoal tool call: create_goal"]
        .into_iter().all(|action| run_ownership_validator(&format!("{table}\nIssue: #461\nBranch: b\nWorktree path: /tmp/w\nPR: #468\n{action}\n")).is_ok_and(|output| !output.status.success()));
    let canonical = format!("{table}| Stop/blocker | None |\n");
    let unseparated = !run_ownership_validator(&format!("{canonical}Issue: #461\nBranch: b\nWorktree path: /tmp/w\nChild branch codexy/461-table was created after classification.\n"))?.status.success();
    assert!(malformed && unseparated);
    Ok(())
}

#[test]
fn validator_allows_inline_child_reads_then_absent_parent_reads()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Lane ownership: child-owned
Implementation-surface reads: Child reads: src/foo; Parent reads: none
Review response: child-authored commit def456 fixed feedback
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should allow inline child reads followed by absent parent reads\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_allows_inline_absent_parent_reads_then_child_reads()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Lane ownership: child-owned
Implementation-surface reads: Parent reads: none; Child reads: src/foo
Review response: child-authored commit def456 fixed feedback
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should allow inline absent parent reads followed by child reads\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_parent_setup_clause_after_absent_read_field()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Lane ownership: child-owned
Parent implementation setup: Parent reads: none; created implementation branch before child delegation
Review response: child-authored commit def456 fixed feedback
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should reject non-read setup evidence after an absent parent-read field"
    );
    Ok(())
}

#[test]
fn validator_allows_absent_parent_setup_clause_after_absent_read_field()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Lane ownership: child-owned
Parent implementation setup: Parent reads: none; no parent-created implementation branch
Review response: child-authored commit def456 fixed feedback
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should allow absent setup evidence after an absent parent-read field\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_mixed_absent_and_present_parent_setup_clause()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Lane ownership: child-owned
Parent implementation setup: Parent reads: none; no parent-created implementation branch but parent-created draft worktree before child delegation
Review response: child-authored commit def456 fixed feedback
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should reject present setup markers hidden in an absent setup clause"
    );
    Ok(())
}

#[test]
fn validator_rejects_direct_mixed_absent_and_present_parent_setup_field()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Lane ownership: child-owned
Parent implementation setup: no parent-created implementation branch but parent-created draft worktree before child delegation
Review response: child-authored commit def456 fixed feedback
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should reject direct setup fields with present setup hidden by leading absence"
    );
    Ok(())
}

#[test]
fn validator_rejects_generic_setup_artifact_after_negated_parent_setup()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Lane ownership: child-owned
Parent implementation setup: no parent implementation setup; created draft worktree before child delegation
Review response: child-authored commit def456 fixed feedback
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should reject generic setup artifacts after negated parent setup text"
    );
    Ok(())
}

#[test]
fn validator_rejects_prose_generic_setup_artifact_after_negated_parent_setup()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Lane ownership: child-owned
No parent implementation setup; created draft worktree before child delegation
Review response: child-authored commit def456 fixed feedback
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should reject prose setup artifacts after negated parent setup text"
    );
    Ok(())
}

#[test]
fn validator_allows_explicit_child_created_setup_artifacts()
-> Result<(), Box<dyn std::error::Error>> {
    for setup_evidence in [
        "Child created implementation branch before starting",
        "Child thread created implementation branch before starting",
        "Child-thread created implementation branch before starting",
        "child-lane created draft worktree before starting",
        "child-thread-created implementation branch before starting",
        "child-lane-created draft worktree before starting",
    ] {
        let output = run_ownership_validator(&format!(
            "Lane ownership: child-owned\n| Task classification | Decision |\n| --- | --- |\n| Lane type | implementation |\n| Secondary surfaces | workflow, validators |\n| Owner decision | current-thread-owned child implementation lane |\n| Atomic scope | issue-sized |\n| Required skills | task-classification, codex-orchestration, git-workflow |\n| Required tools/evidence | goal, plan, codegraph, LSP, Sentinel |\n| First allowed action | create branch after classification |\n| Stop/blocker | None |\n\n{setup_evidence}\nReview response: child-authored commit def456 fixed feedback\nMaintainer reassignment: none\n"
        ))?;

        assert!(
            output.status.success(),
            "validator should not classify explicit child setup as parent setup: {setup_evidence}\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[test]
fn validator_rejects_parent_setup_when_recovery_is_empty_before_stop_condition()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Lane ownership: child-owned
Parent implementation setup: created draft worktree before child delegation
Recovery:
Stop condition: disclose the workflow defect, preserve the diff, inspect user overlap, and delegate to a clean child thread
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should stop empty recovery continuations at unlisted metadata"
    );
    Ok(())
}
