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
fn validator_allows_replacement_when_count_lists_old_owner_after_other_thread()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for orchestration only; child routing required
Child thread thread-old stopped and freed replacement capacity.
Active child Codex threads: 5, including thread-other for issue #270, thread-old for issue #269.
Existing issue/PR owner check: existing owner thread thread-old found for issue #269.
Old owner disposition: existing owner thread thread-old was stopped as unusable for issue #269.
Thread creation: created replacement child thread thread-new for issue #269.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should match a replacement owner anywhere in the active-count line\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_allows_first_child_thread_after_empty_ledger_count()
-> Result<(), Box<dyn std::error::Error>> {
    for count in ["none", "zero"] {
        let output = run_ownership_validator(&format!(
            r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: {count}
Existing issue/PR owner check: no existing owner thread found for issue #269.
Thread creation: created child thread thread-269 for issue #269.
Maintainer reassignment: none
"#
        ))?;

        assert!(
            output.status.success(),
            "validator should treat empty active ledger value {count:?} as count zero\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[test]
fn validator_does_not_double_count_active_waiting_aggregate_with_component()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for orchestration only; child routing required
Active/waiting child Codex threads: 5
Waiting child Codex threads: 1
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should not add component waiting count to an active/waiting aggregate total\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_complete_component_counts_after_active_waiting_aggregate()
-> Result<(), Box<dyn std::error::Error>> {
    for counts in [
        "Active child Codex threads: 3\nWaiting child Codex threads: 3",
        "Waiting child Codex threads: 3\nActive child Codex threads: 3",
    ] {
        let output = run_ownership_validator(&format!(
            r#"Owner decision: parent-owned for orchestration only; child routing required
Active/waiting child Codex threads: 5
{counts}
Maintainer reassignment: none
"#
        ))?;

        assert!(
            !output.status.success(),
            "validator should treat complete later component counts as a newer breakdown"
        );
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .contains("keep at most five active child Codex threads"),
            "stderr should name over-capacity components, got:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[test]
fn validator_projects_creation_from_active_waiting_aggregate_not_component_sum()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for orchestration only; child routing required
Active/waiting child Codex threads: 4
Waiting child Codex threads: 1
Existing issue/PR owner check: no existing owner thread found for issue #269.
Thread creation: created child thread thread-269 for issue #269.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should project creation from aggregate active/waiting total, not active-plus-waiting double count\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_creation_after_complete_components_follow_aggregate()
-> Result<(), Box<dyn std::error::Error>> {
    for counts in [
        "Active child Codex threads: 4\nWaiting child Codex threads: 1",
        "Waiting child Codex threads: 1\nActive child Codex threads: 4",
    ] {
        let output = run_ownership_validator(&format!(
            r#"Owner decision: parent-owned for orchestration only; child routing required
Active/waiting child Codex threads: 5
{counts}
Existing issue/PR owner check: no existing owner thread found for issue #269.
Thread creation: created child thread thread-269 for issue #269.
Maintainer reassignment: none
"#
        ))?;

        assert!(
            !output.status.success(),
            "validator should project from complete later component counts"
        );
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .contains("would exceed five active child Codex threads"),
            "stderr should name over-capacity creation, got:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[test]
fn validator_rejects_creation_after_full_active_count_with_zero_waiting()
-> Result<(), Box<dyn std::error::Error>> {
    for count in ["zero", "none"] {
        let output = run_ownership_validator(&format!(
            r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: 5, {count} waiting
Existing issue/PR owner check: no existing owner thread found for issue #269.
Thread creation: created child thread thread-269 for issue #269.
Maintainer reassignment: none
"#
        ))?;

        assert!(
            !output.status.success(),
            "validator should not treat mixed {count:?} waiting wording as an empty active ledger"
        );
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .contains("would exceed five active child Codex threads"),
            "stderr should name over-capacity creation, got:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}
