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
fn validator_allows_one_defect_capture_for_same_line_handler_failures()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.list_threads and codex_app.list_projects as available thread tools.
Invocation evidence: codex_app.list_threads failed with `No handler registered for tool: list_threads`; codex_app.list_projects failed with `No handler registered for tool: list_projects`.
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.list_threads and codex_app.list_projects.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        output.status.success(),
        "validator should accept one following defect report for same-line handler failures\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}
