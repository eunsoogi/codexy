#![cfg(unix)]

use std::{
    process::{Child, Command, Stdio},
    thread,
    time::Duration,
};

use codexy_runtime::child_monitoring::{
    AwaitedGate, ChildLocalMonitor, GateOutcome, ObservationEffect, ParentDelta,
};

fn launch_live_sentinel(verdict: &str) -> Child {
    Command::new("sh")
        .args([
            "-c",
            "sleep 0.2; printf '%s\\n' \"$1\"",
            "sentinel",
            verdict,
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap()
}

#[test]
fn runtime_monitor_preserves_live_pass_and_block_delivery() {
    for verdict in ["PASS", "BLOCK"] {
        let mut sentinel = launch_live_sentinel(verdict);
        let mut monitor = ChildLocalMonitor::new(AwaitedGate::Sentinel, 2).unwrap();
        assert!(matches!(
            monitor.observe_process_liveness(&mut sentinel).unwrap(),
            ObservationEffect::RetainActive
        ));
        thread::sleep(Duration::from_millis(50));
        assert!(matches!(
            monitor.observe_process_liveness(&mut sentinel).unwrap(),
            ObservationEffect::RetainActive
        ));
        assert!(monitor.goal_is_active());
        assert!(monitor.plan_is_awaiting());

        let output = sentinel.wait_with_output().unwrap();
        assert_eq!(
            output.status.code(),
            Some(0),
            "status observation interrupted {verdict}"
        );
        assert_eq!(
            String::from_utf8(output.stdout).unwrap(),
            format!("{verdict}\n")
        );
        let outcome = match verdict {
            "PASS" => GateOutcome::Passed,
            "BLOCK" => GateOutcome::Blocked,
            _ => unreachable!(),
        };
        assert_eq!(
            monitor.observe_terminal(outcome),
            Some(ParentDelta {
                gate: AwaitedGate::Sentinel,
                outcome,
            })
        );
    }
}

#[test]
fn terminal_process_observation_defers_the_only_parent_delta() {
    let mut sentinel = Command::new("sh")
        .args(["-c", "exit 0"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut monitor = ChildLocalMonitor::new(AwaitedGate::Sentinel, 1).unwrap();

    thread::sleep(Duration::from_millis(50));
    assert_eq!(
        monitor.observe_process_liveness(&mut sentinel).unwrap(),
        ObservationEffect::Terminal
    );
    assert!(monitor.goal_is_active());
    assert_eq!(
        monitor.observe_terminal(GateOutcome::Passed),
        Some(ParentDelta {
            gate: AwaitedGate::Sentinel,
            outcome: GateOutcome::Passed,
        })
    );
}

#[test]
fn proof_completion_policy_excludes_live_sentinel_from_polling_and_followups() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let policy = std::fs::read_to_string(
        root.join("plugins/codexy/skills/proof-driven-completion/SKILL.md"),
    )
    .unwrap();

    assert!(
        policy.contains("For a running packaged Sentinel, parent observation MUST be read-only.")
    );
    assert!(policy.contains("MUST NOT poll, send messages, interrupts, or follow-up prompts."));
}
