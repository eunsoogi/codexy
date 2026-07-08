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
fn validator_ignores_not_yet_been_passive_launch_negations()
-> Result<(), Box<dyn std::error::Error>> {
    for operation in [
        "Child thread thread-269 has not yet been started for issue #269.",
        "Child thread thread-269 has not yet been forked for issue #269.",
        "Child thread thread-269 hasn't yet been requested for issue #269.",
        "Child thread has not yet been created for issue #269.",
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
            output.status.success(),
            "validator should ignore negated passive launch `{operation}`\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[test]
fn validator_rejects_id_before_child_thread_launches_over_active_cap()
-> Result<(), Box<dyn std::error::Error>> {
    for operation in [
        "Thread thread-269 was started as a child thread for issue #269.",
        "Thread thread-269 forked as a child thread for issue #269.",
        "Started thread-269 as a child thread for issue #269.",
        "Requested thread-269 as a child thread for issue #269.",
        "Started as a child thread for issue #269.",
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
            "validator should treat id-before-verb launch `{operation}` as a child-thread operation"
        );
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .contains("would exceed five active child Codex threads"),
            "stderr should name over-capacity launch `{operation}`, got:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}
