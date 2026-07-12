#![cfg(unix)]

use std::{
    io,
    process::{Child, Command, Stdio},
    thread,
    time::Duration,
};

enum SentinelState {
    Running,
    Terminal,
}

struct StatusObserver;

impl StatusObserver {
    fn observe(&self, sentinel: &mut Child) -> io::Result<SentinelState> {
        Ok(match sentinel.try_wait()? {
            Some(_) => SentinelState::Terminal,
            None => SentinelState::Running,
        })
    }
}

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
fn read_only_status_observation_preserves_live_pass_and_block_delivery() {
    let observer = StatusObserver;

    for verdict in ["PASS", "BLOCK"] {
        let mut sentinel = launch_live_sentinel(verdict);
        assert!(matches!(
            observer.observe(&mut sentinel).unwrap(),
            SentinelState::Running
        ));
        thread::sleep(Duration::from_millis(50));
        assert!(matches!(
            observer.observe(&mut sentinel).unwrap(),
            SentinelState::Running
        ));

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
    }
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
