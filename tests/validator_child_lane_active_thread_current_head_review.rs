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
fn validator_allows_thread_only_owner_lookup_for_resume() -> Result<(), Box<dyn std::error::Error>>
{
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: 5
Existing issue/PR owner check: existing owner thread thread-148 found.
Thread resume: send_message_to_thread(thread_id="thread-148").
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should accept thread-id-only owner lookup before a thread resume\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_preserves_batched_no_owner_lookups_for_independent_lanes()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: 3
Existing issue/PR owner check: no existing owner thread found for issue #269.
Existing issue/PR owner check: no existing owner thread found for issue #270.
Thread creation: created child thread thread-269 for issue #269.
Thread creation: created child thread thread-270 for issue #270.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should keep independent preflight lookups available for later independent operations\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_ignores_thread_tools_explicitly_not_called() -> Result<(), Box<dyn std::error::Error>>
{
    for evidence in [
        r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: 5
Thread tools: codex_app.create_thread was not called for issue #269.
Maintainer reassignment: none
"#,
        r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: 5
Thread tools: create_thread(thread_id="thread-269") was not called for issue #269.
Maintainer reassignment: none
"#,
        r#"Owner decision: parent-owned for orchestration only; child routing required
Active child Codex threads: 5
Thread tools: send_message_to_thread(thread_id="thread-148") wasn't called.
Maintainer reassignment: none
"#,
    ] {
        let output = run_ownership_validator(evidence)?;

        assert!(
            output.status.success(),
            "validator should ignore explicitly not-called thread tools for evidence:\n{evidence}\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}
