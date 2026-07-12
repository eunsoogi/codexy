use std::path::Path;

fn policy(path: &str) -> String {
    std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(path)).unwrap()
}

fn normalized_policy(path: &str) -> String {
    policy(path)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[test]
fn status_observation_does_not_mutate_a_live_sentinel() {
    let orchestration = normalized_policy("plugins/codexy/skills/codex-orchestration/SKILL.md");

    assert!(
        orchestration
            .contains("Status observation of a running packaged Sentinel MUST be read-only.")
    );
    assert!(
        orchestration
            .contains("MUST NOT send messages, interrupts, follow-up prompts, or other mutations.")
    );
    assert!(orchestration.contains(
        "A live Sentinel MUST remain active until it produces its own `PASS`, `BLOCK`, or `UNOBSERVABLE` terminal result;"
    ));
}

#[test]
fn delayed_sentinel_output_preserves_terminal_delivery() {
    let control = normalized_policy(
        "plugins/codexy/skills/codex-orchestration/references/classification-and-control.md",
    );
    let events = normalized_policy("plugins/codexy/skills/token-efficient-orchestration/SKILL.md");

    assert!(control.contains("Delayed output alone MUST NOT cause `UNOBSERVABLE`."));
    assert!(events.contains("only material terminal deltas"));
    assert!(events.contains("MUST NOT poll a running Sentinel"));
}
