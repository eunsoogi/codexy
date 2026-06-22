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
fn validator_rejects_colonized_subagent_owner_continuation()
-> Result<(), Box<dyn std::error::Error>> {
    for evidence in [
        r#"Owner decision: child-owned implementation lane
Subthread/worktree owner:
- multi_agent_v1: subagent Gauss
Parent implementation setup: none
Maintainer reassignment: none
"#,
        r#"Owner decision: child-owned implementation lane
Subthread/worktree owner:
multi_agent_v1: subagent Gauss
Parent implementation setup: none
Maintainer reassignment: none
"#,
    ] {
        let output = run_ownership_validator(evidence)?;

        assert!(
            !output.status.success(),
            "validator should reject colonized subagent continuations under an owner field\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}
