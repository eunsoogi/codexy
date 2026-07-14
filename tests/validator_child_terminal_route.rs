use std::process::{Command, Output};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn validator_rejects_local_parent_routes_in_terminal_only_handoffs() -> TestResult {
    for route in ["agents.send_message('/root')", "/root", "agents.worker"] {
        let local = run_validator(&terminal_only_evidence(route))?;
        assert!(
            !local.status.success(),
            "terminal-only handoff must reject local parent route {route:?}"
        );
        assert!(
            String::from_utf8_lossy(&local.stderr)
                .contains("child goal reporting must not use local agents /root routing")
        );
    }

    let codex = run_validator(&terminal_only_evidence("codex task/thread"))?;
    assert!(
        codex.status.success(),
        "Codex task/thread route must remain valid: {}",
        String::from_utf8_lossy(&codex.stderr)
    );
    Ok(())
}

fn terminal_only_evidence(parent_route: &str) -> String {
    format!(
        "Lane ownership: child-owned\nTerminal parent handoff: event id=terminal-child|417|archive; issue/pr=#417 / PR #418; child task=child-417; parent task=019f49da-d44c-7e41-afde-8b1f7c58efa0; branch=eunsoogi/417-distinguish-runtime-monitors; worktree=/worktree; head=abc; clean/index=clean; last proof=focused validator; current gate=parent review; preserved reservation/artifacts=worktree reserved; parent next action=inspect the PR; delivery=confirmed; task surface=codex task/thread\nParent route: {parent_route}\nTerminal child transition: action=archive\n"
    )
}

fn run_validator(evidence: &str) -> Result<Output, Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let evidence_path = temp.path().join("handoff.md");
    std::fs::write(&evidence_path, evidence)?;
    Ok(Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args(["--check-child-lane-ownership", "--evidence-file"])
        .arg(&evidence_path)
        .output()?)
}
