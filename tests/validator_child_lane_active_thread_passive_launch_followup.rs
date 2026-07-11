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
fn validator_rejects_issue_only_passive_started_forked_requested_child_threads_over_active_cap()
-> Result<(), Box<dyn std::error::Error>> {
    for operation in [
        "Child thread was started for issue #269.",
        "Child thread was forked for issue #269.",
        "Child thread was requested for issue #269.",
    ] {
        let output = run_ownership_validator(&format!(
            r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: 5
Existing issue/PR owner check: no existing owner thread found for issue #269.
{operation}
Maintainer reassignment: none
"#
        ))?;

        assert!(
            !output.status.success(),
            "validator should treat issue-only passive operation `{operation}` as a child-thread launch"
        );
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .contains("would exceed five active child Codex threads"),
            "stderr should name over-capacity issue-only passive operation `{operation}`, got:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[test]
fn validator_rejects_passive_launch_with_unrelated_later_negation_over_active_cap()
-> Result<(), Box<dyn std::error::Error>> {
    for operation in [
        "Child thread started: thread-269 for issue #269, did not create a duplicate owner.",
        "Child thread thread-269 was started for issue #269, did not create a duplicate owner.",
    ] {
        let output = run_ownership_validator(&format!(
            r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: 5
Existing issue/PR owner check: no existing owner thread found for issue #269.
{operation}
Maintainer reassignment: none
"#
        ))?;

        assert!(
            !output.status.success(),
            "validator should not let unrelated later negation hide passive launch `{operation}`"
        );
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .contains("would exceed five active child Codex threads"),
            "stderr should name over-capacity passive launch `{operation}`, got:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}
