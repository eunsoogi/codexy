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

fn evidence_with_defect_phrase(phrase: &str) -> String {
    format!(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Invocation evidence: codex_app.read_thread failed with `no handler registered for tool: read_thread`.
Dogfooding/tool-exposure defect {phrase}: recorded runtime missing-handler evidence for codex_app.read_thread.
Fallback route: no fallback route was available.
Tracking issue: #205.
Maintainer reassignment: none
"#
    )
}

#[test]
fn validator_rejects_ambiguous_multi_lane_defect_capture() -> Result<(), Box<dyn std::error::Error>>
{
    for phrase in [
        "for Lane A and B",
        "for Lane A or B",
        "for Lane A and/or B",
        "for Lane A and-or B",
        "for Lane A, B",
        "for Lane A/B",
        "for Lanes Alpha and Beta",
        "for Lane Alpha and Beta",
        "for Lane Alpha or Beta",
        "for Lane Alpha and/or Beta",
        "for Lane Alpha and-or Beta",
        "for Lane Alpha, Beta",
        "for Lane Alpha/Beta",
    ] {
        let output = run_ownership_validator(&evidence_with_defect_phrase(phrase))?;

        assert!(
            !output.status.success(),
            "validator should reject multi-lane capture {phrase}\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[test]
fn validator_allows_conjunction_prose_after_explicit_lane_capture()
-> Result<(), Box<dyn std::error::Error>> {
    for phrase in [
        "for Lane A and I recorded the handoff",
        "for Lane A or I can provide the evidence",
        "for Lane A and/or I can follow up",
        "for Lane A and-or I can follow up",
        "for Lane A and we recorded the handoff",
        "for Lane Alpha and we recorded the handoff",
        "for Lane Alpha and work continued",
        "for Lane Alpha or ordinary handoff prose",
        "for Lane Alpha and the owner recorded the handoff",
        "for Lane A or ordinary handoff prose",
    ] {
        let output = run_ownership_validator(&evidence_with_defect_phrase(phrase))?;

        assert!(
            output.status.success(),
            "validator should not treat conjunction prose as another lane for {phrase}\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[test]
fn validator_preserves_lane_scope_after_length_changing_unicode()
-> Result<(), Box<dyn std::error::Error>> {
    let prefixed = |phrase: &str| {
        format!(
            r"İ note
Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Fallback route: no fallback route was available.
Tracking issue: #205.
Invocation evidence: codex_app.read_thread failed with `no handler registered for tool: read_thread`.
Dogfooding/tool-exposure defect {phrase}: recorded runtime missing-handler evidence for codex_app.read_thread."
        )
    };

    let multi_lane = run_ownership_validator(&prefixed("for Lane Alpha and Beta"))?;
    assert!(
        !multi_lane.status.success(),
        "validator should reject a Unicode-prefixed multi-lane capture\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&multi_lane.stdout),
        String::from_utf8_lossy(&multi_lane.stderr)
    );

    let prose = run_ownership_validator(&prefixed("for Lane Alpha and work continued"))?;
    assert!(
        prose.status.success(),
        "validator should preserve Unicode-prefixed prose\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&prose.stdout),
        String::from_utf8_lossy(&prose.stderr)
    );
    Ok(())
}
