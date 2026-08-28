use std::fs;

use crate::support::TestResult;

#[test]
fn sentinel_handoff_keeps_direct_state_without_legacy_artifacts() -> TestResult {
    let path = codexy_runtime::paths::repository_root()
        .join("plugins/codexy/agents/codexy-sentinel.toml");
    let text = fs::read_to_string(path)?;
    let handoff = text
        .split_once("Selected terminal handoff:")
        .map(|(_, rest)| rest)
        .expect("selected terminal handoff");
    let forbidden = [
        ["codexy", "review", "terminal-record", "v1"],
        ["codexy", "review", "ledger", "v1"],
        ["codexy", "review", "packet", "v4"],
    ];
    for parts in forbidden {
        let forbidden = parts.join(".");
        assert!(
            !handoff.contains(&forbidden),
            "selected handoff must not require {forbidden}"
        );
    }
    for required in [
        "selected profile",
        "selected reviewer",
        "exact current head",
        "PASS",
        "BLOCK",
        "UNOBSERVABLE",
        "unresolved finding",
        "full",
        "delta",
        "connector_repair",
    ] {
        assert!(
            handoff.to_ascii_lowercase().contains(&required.to_ascii_lowercase()),
            "selected handoff must retain direct-state field {required}"
        );
    }
    Ok(())
}
