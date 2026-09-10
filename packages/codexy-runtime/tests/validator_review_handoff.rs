use std::fs;

use crate::support::TestResult;

#[test]
fn sentinel_handoff_keeps_direct_state_without_legacy_artifacts() -> TestResult {
    let path = codexy_runtime::paths::repository_root()
        .join("plugins/codexy/agents/codexy-sentinel.toml");
    let text = fs::read_to_string(path)?;
    let (compact, legacy) = text
        .split_once("Compact terminal handoff:")
        .map(|(_, rest)| rest)
        .and_then(|rest| rest.split_once("Explicit legacy handoff:"))
        .expect("compact and explicit legacy handoffs");
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
        "full",
        "delta",
        "terminal_review_count",
        "terminal_review_limit",
        "terminal_review_history",
        "required_current_head",
        "mandatory_base_integration",
        "in_scope_contract_root_repair",
        "qualifying_change",
        "evidence commit",
        "persisted prior PR snapshot",
    ] {
        assert!(
            legacy.to_ascii_lowercase().contains(&required.to_ascii_lowercase()),
            "legacy handoff must retain direct-state field {required}"
        );
    }
    Ok(())
}
