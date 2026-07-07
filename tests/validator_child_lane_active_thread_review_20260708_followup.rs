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
fn validator_rejects_article_separated_fork_start_operations_over_cap()
-> Result<(), Box<dyn std::error::Error>> {
    for operation in [
        "Forked a child thread thread-new for issue #269.",
        "Started a child thread thread-new for issue #269.",
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
            "validator should detect article-separated child-thread operation: {operation}"
        );
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .contains("would exceed five active child Codex threads"),
            "stderr should name over-capacity operation, got:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[test]
fn validator_rejects_active_waiting_total_over_cap() -> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for orchestration only; child routing required
Active/waiting child Codex threads: 6
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should treat active/waiting totals as cap counts"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("keep at most five active child Codex threads"),
        "stderr should name over-capacity active/waiting total, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}
