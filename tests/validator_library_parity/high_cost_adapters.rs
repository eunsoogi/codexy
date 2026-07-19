use std::path::Path;

use crate::support;

#[test]
fn high_cost_validator_suites_route_checked_fixtures_through_the_library()
-> Result<(), Box<dyn std::error::Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    for (relative, adapter) in [
        (
            "tests/validator_instruction_policy.rs",
            "validator_instruction_policy",
        ),
        (
            "tests/validator_instruction_policy_passive.rs",
            "validator_instruction_policy",
        ),
        (
            "tests/validator_gpt_5_6_routing_adversarial.rs",
            "routing_validator::",
        ),
        (
            "tests/validator_gpt_5_6_routing_boundaries.rs",
            "routing_validator::",
        ),
        (
            "tests/validator_gpt_5_6_routing_contextual.rs",
            "routing_validator::",
        ),
        (
            "tests/validator_gpt_5_6_routing_field_semantics.rs",
            "routing_validator::",
        ),
        (
            "tests/validator_gpt_5_6_routing_review_feedback.rs",
            "routing_validator::",
        ),
        (
            "tests/validator_sentinel_scope_policy.rs",
            "support::validator",
        ),
        (
            "tests/validator_runtime_heartbeat_contract.rs",
            "support::validator",
        ),
        (
            "tests/validator_child_external_gate_policy.rs",
            "validator_instruction_policy",
        ),
        (
            "tests/validator_execution_budget_policy.rs",
            "validator_instruction_policy",
        ),
        (
            "tests/validator_live_worktree_reservation_preflight.rs",
            "validator_instruction_policy",
        ),
    ] {
        let source = std::fs::read_to_string(root.join(relative))?;
        support::assert_structured_literals(
            &source,
            "high-cost validator library adapter",
            &[adapter],
        );
        if source.contains("CARGO_BIN_EXE_codexy-validate") {
            return Err(format!("{relative} must use the focused library adapter").into());
        }
    }
    for entry in std::fs::read_dir(root.join("tests"))? {
        let file_path = entry?.path();
        let name = file_path.file_name().map(|name| name.to_string_lossy());
        if name.as_deref().is_some_and(|name| {
            name.starts_with("validator_runtime_heartbeat_")
                && name != "validator_runtime_heartbeat_reference_registration.rs"
        }) {
            let source = std::fs::read_to_string(&file_path)?;
            support::assert_structured_literals(
                &source,
                "runtime heartbeat focused validator adapter",
                &["validator_instruction_policy"],
            );
        }
    }
    Ok(())
}
