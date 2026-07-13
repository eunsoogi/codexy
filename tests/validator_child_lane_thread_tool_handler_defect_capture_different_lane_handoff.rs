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
fn validator_rejects_preceding_fallback_metadata_scoped_to_other_lane_phrases()
-> Result<(), Box<dyn std::error::Error>> {
    for phrase in [
        "another lane",
        "different lane",
        "later lane",
        "other lane",
        "Lane B",
    ] {
        let route_scope = if phrase == "Lane B" {
            "for Lane B".to_string()
        } else {
            format!("for {phrase}")
        };
        let output = run_ownership_validator(&format!(
            r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Lane A:
Fallback route: no fallback route was available {route_scope}.
Tracking issue: #246
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread in Lane A.
Maintainer reassignment: none
"#
        ))?;

        assert!(
            !output.status.success(),
            "validator should drop fallback metadata scoped to {phrase} before preserving Lane A defect scope\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[test]
fn validator_rejects_preceding_fallback_metadata_with_later_different_lane_mention()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_ownership_validator(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Lane A:
Fallback route: no fallback route was available for Lane A and in Lane B review thread.
Tracking issue: #246
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread in Lane A.
Maintainer reassignment: none
"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should reject metadata that mentions Lane A first but later scopes the same fallback to Lane B\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}
