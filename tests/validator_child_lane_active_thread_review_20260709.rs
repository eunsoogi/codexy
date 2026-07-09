use std::process::{Command, Output};

fn run_ownership_validator(evidence: &str) -> Result<Output, Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let evidence_path = temp.path().join("handoff.md");
    std::fs::write(&evidence_path, evidence)?;

    Ok(Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args(["--check-child-lane-ownership", "--evidence-file"])
        .arg(&evidence_path)
        .output()?)
}

#[test]
fn validator_rejects_reordered_passive_launches_joined_by_and()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: 4
Existing issue/PR owner check: no existing owner thread found for issue #269.
Existing issue/PR owner check: no existing owner thread found for issue #270.
Started thread-269 as a child thread for issue #269 and started thread-270 as a child thread for issue #270.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should count both reordered passive child-thread launches"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("would exceed five active child Codex threads"),
        "stderr should name over-capacity reordered passive launch, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_ignores_child_thread_request_not_yet_made() -> Result<(), Box<dyn std::error::Error>> {
    for line in [
        "Child thread request has not yet been made for issue #269.",
        "Child thread request hasn't yet been made for issue #269.",
    ] {
        let output = run_ownership_validator(&format!(
            r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: 5
{line}
Maintainer reassignment: none
"#
        ))?;

        assert!(
            output.status.success(),
            "validator should ignore nominal child-thread request not yet made `{line}`, got:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[test]
fn validator_rejects_replacement_when_latest_count_no_longer_names_old_owner()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for orchestration only; child routing required
Child thread thread-old stopped and freed replacement capacity.
Active child Codex threads: 4, including thread-old for issue #269.
Active child Codex threads: 5, including thread-101, thread-102, thread-103, thread-104, thread-105.
Existing issue/PR owner check: existing owner thread thread-old found for issue #269.
Old owner disposition: existing owner thread thread-old was stopped as unusable for issue #269.
Thread creation: created replacement child thread thread-new for issue #269.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should use the latest count when deciding replacement capacity neutrality"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("would exceed five active child Codex threads"),
        "stderr should name over-capacity replacement, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_allows_owner_lookup_with_unrelated_same_line_no_evidence_note()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: 4
Existing issue/PR owner check: no existing owner thread found for issue #269, Review blockers: no evidence of blockers.
Thread creation: created child thread thread-269 for issue #269.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should scope owner-check negation to lookup clauses, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_counts_thread_ids_without_counting_branch_names()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: thread-101 on branch codex-269, thread-102 on branch codex-270, thread-103 on branch codex-271
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should count thread IDs without treating branch names as thread IDs, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_keeps_explicit_total_with_branch_names() -> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: thread-101 on branch codex-269, thread-102 on branch codex-270, thread-103 on branch codex-271 (3 total)
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should prefer explicit totals over nearby branch/worktree tokens, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}
