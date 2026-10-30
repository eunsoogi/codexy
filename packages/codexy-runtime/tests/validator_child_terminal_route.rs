//! Ensures terminal receipts use the Codex task/thread route rather than local
//! agent messaging or helper routes.

use std::process::Output;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn validator_rejects_local_parent_routes_in_terminal_only_handoffs() -> TestResult {
    for route in ["agents.send_message('/root')", "/root", "agents.worker"] {
        let local = run_validator(&terminal_only_evidence(route))?;
        assert!(
            !local.status.success(),
            "terminal-only handoff must reject local parent route {route:?}"
        );
        let table = terminal_only_evidence("codex task/thread").replace(
            "Parent route: codex task/thread", &format!("Parent route:\n| Field | Value |\n| --- | --- |\n| Target | {route} |"),
        );
        assert!(!run_validator(&table)?.status.success(), "table route must reject {route}");
        assert!(
            String::from_utf8_lossy(&local.stderr)
                .contains("terminal child handoff must not use local agents /root routing")
        );
    }

    let codex = run_validator(&terminal_only_evidence("codex task/thread"))?;
    assert!(
        codex.status.success(),
        "Codex task/thread route must remain valid: {}",
        String::from_utf8_lossy(&codex.stderr)
    );
    let table = terminal_only_evidence("codex task/thread").replace(
        "Parent route: codex task/thread",
        "Parent route:\n| Field | Value |\n| --- | --- |\n| Target | codex task/thread |",
    );
    let valid = run_validator(&table)?;
    assert!(valid.status.success(), "{}", String::from_utf8_lossy(&valid.stderr));
    Ok(())
}

#[test]
fn conflicting_terminal_actions_never_bypass_handoff() -> TestResult {
    for transition in [
        "Terminal child transition: action=archive; action=stop\n",
        "Terminal child transition:\n| Field | Value |\n| --- | --- |\n| Action | archive |\n| Action | stop |\n",
    ] {
        let evidence = format!("Lane ownership: child-owned\n{transition}");
        assert!(!run_validator(&evidence)?.status.success(), "{evidence}");
    }
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
    crate::support::validator_child_lane_ownership_file(&evidence_path)
}
