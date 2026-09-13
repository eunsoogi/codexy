use std::fs;

use crate::support::TestResult;

#[test]
fn sentinel_handoff_keeps_direct_state_without_legacy_artifacts() -> TestResult {
    let path = codexy_runtime::paths::repository_root()
        .join("plugins/codexy/agents/codexy-sentinel.toml");
    let text = fs::read_to_string(path)?;
    let (compact, retired) = text
        .split_once("Compact terminal handoff:")
        .map(|(_, rest)| rest)
        .and_then(|rest| rest.split_once("Retired review-count"))
        .expect("compact and retired handoffs");
    let forbidden = [
        ["codexy", "review", "terminal-record", "v1"],
        ["codexy", "review", "ledger", "v1"],
        ["codexy", "review", "packet", "v4"],
    ];
    for parts in forbidden {
        let forbidden = parts.join(".");
        assert!(
            !compact.contains(&forbidden),
            "compact handoff must not require {forbidden}"
        );
    }
    for required in [
        "selected profile",
        "policy reviewer",
        "exact current head",
        "PASS",
        "BLOCK",
        "UNOBSERVABLE",
        "unresolved finding",
    ] {
        assert!(
            compact.to_ascii_lowercase().contains(&required.to_ascii_lowercase()),
            "compact handoff must retain current-head field {required}"
        );
    }
    for required in [
        "unsupported by the runtime",
        "rejected before compact",
        "immutable",
        "MUST NOT be executed",
        "used to establish current-head readiness",
    ] {
        assert!(
            retired.to_ascii_lowercase().contains(&required.to_ascii_lowercase()),
            "retired handoff must retain boundary statement {required}"
        );
    }
    Ok(())
}
