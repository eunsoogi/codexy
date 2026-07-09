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
fn validator_ignores_discovered_create_thread_signature() -> Result<(), Box<dyn std::error::Error>>
{
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.create_thread(title="Codexy #269 lane") as an available thread tool.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should not treat tool discovery signatures as child-thread launches, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_skipped_owner_check_before_not_found_result()
-> Result<(), Box<dyn std::error::Error>> {
    for line in [
        "Existing issue/PR owner check: did not check; no existing owner thread found for issue #269.",
        "Existing issue/PR owner check: didn't check; no existing owner thread found for issue #269.",
    ] {
        let output = run_ownership_validator(&format!(
            r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: 4
{line}
Thread creation: created child thread thread-269 for issue #269.
Maintainer reassignment: none
"#
        ))?;

        assert!(
            !output.status.success(),
            "validator should reject skipped owner-check wording before not-found lookup `{line}`"
        );
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("existing issue/PR owner thread"),
            "stderr should require valid owner-check evidence, got:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}
