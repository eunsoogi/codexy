use std::path::Path;

use crate::support;

#[test]
fn high_cost_validator_suites_route_checked_fixtures_through_the_library()
-> Result<(), Box<dyn std::error::Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    for (relative, adapter) in [
        (
            "tests/validator_sentinel_reviewer_gate.rs",
            "support::validator_in_process",
        ),
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
        if relative == "tests/validator_instruction_policy.rs" {
            support::assert_structured_literals(
                &source,
                "instruction policy copy-on-write fixture",
                &[
                    "support::copy_plugin_fixture_with_mutable_files",
                    "agents/codexy-sculptor.toml",
                ],
            );
        }
        if relative == "tests/validator_sentinel_reviewer_gate.rs" {
            support::assert_structured_literals(
                &source,
                "sentinel reviewer manifest-aware fixture",
                &["support::roles_fixture"],
            );
        }
    }
    for (relative, mutable_file) in [
        (
            "tests/validator_gpt_5_6_routing.rs",
            "skills/codex-orchestration/SKILL.md",
        ),
        (
            "tests/validator_role_instruction_policy.rs",
            "agents/codexy-sentinel.toml",
        ),
    ] {
        let source = std::fs::read_to_string(root.join(relative))?;
        support::assert_structured_literals(
            &source,
            "high-cost copy-on-write fixture",
            &[
                "support::copy_plugin_fixture_into_with_mutable_files",
                mutable_file,
            ],
        );
    }
    for (relative, fixture_api, mutable_file) in [
        (
            "tests/validator_connector_review_policy_markdown_boundaries.rs",
            "support::copy_plugin_fixture_with_mutable_files",
            "skills/git-workflow/references/codex-connector-review.md",
        ),
        (
            "tests/validator_execution_budget_policy_controls.rs",
            "support::copy_plugin_fixture_with_mutable_files",
            "skills/codex-orchestration/references/execution-budget.md",
        ),
        (
            "tests/validator_parent_execution_budget_countermands.rs",
            "support::copy_plugin_fixture_with_mutable_files",
            "skills/codex-orchestration/references/execution-budget.md",
        ),
        (
            "tests/validator_parent_execution_budget_policy.rs",
            "support::copy_plugin_fixture_with_mutable_files",
            "skills/codex-orchestration/references/execution-budget.md",
        ),
        (
            "tests/validator_token_polling_runtime_identity.rs",
            "support::copy_plugin_fixture_with_mutable_files",
            "skills/token-efficient-orchestration/SKILL.md",
        ),
        (
            "tests/token_quota_containment.rs",
            "support::copy_plugin_fixture_with_mutable_files",
            "skills/codex-orchestration/SKILL.md",
        ),
        (
            "tests/validator_runtime_heartbeat_wait_priority.rs",
            "support::plugin_fixture_with_mutable_files",
            "skills/codex-orchestration/references/runtime-heartbeats.md",
        ),
    ] {
        let source = std::fs::read_to_string(root.join(relative))?;
        support::assert_structured_literals(
            &source,
            "high-cost declared-mutable fixture",
            &[fixture_api, mutable_file],
        );
    }
    let heartbeat_fixture = std::fs::read_to_string(
        root.join("tests/validator_runtime_heartbeat_contract/fixture.rs"),
    )?;
    support::assert_structured_literals(
        &heartbeat_fixture,
        "runtime heartbeat declared-mutable fixture",
        &["support::plugin_fixture_with_mutable_files"],
    );
    let heartbeat_contract = std::fs::read_to_string(root.join(
        "tests/validator_runtime_heartbeat_contract.rs",
    ))?;
    support::assert_structured_literals(
        &heartbeat_contract,
        "runtime heartbeat fixture mutation path",
        &["skills/codex-orchestration/references/runtime-heartbeats.md"],
    );
    for (relative, mutable_file) in [
        (
            "tests/validator_agent_registration_bootstrap.rs",
            "skills/codex-orchestration/scripts/bootstrap-codexy-agents",
        ),
        (
            "tests/validator_agent_registration_bootstrap_security.rs",
            "",
        ),
        ("tests/validator_agent_registration_hardening.rs", ""),
        ("tests/validator_agent_registration_transactions.rs", ""),
        ("tests/validator_agent_registration.rs", ""),
    ] {
        let source = std::fs::read_to_string(root.join(relative))?;
        support::assert_structured_literals(
            &source,
            "agent registration copy-on-write fixture",
            &["support::copy_plugin_fixture_into_with_mutable_files"],
        );
        if !mutable_file.is_empty() {
            support::assert_structured_literals(
                &source,
                "agent registration mutable fixture file",
                &[mutable_file],
            );
        }
        if source.contains("support::copy_dir(") {
            return Err(format!("{relative} must use the shared copy-on-write fixture").into());
        }
    }
    for entry in std::fs::read_dir(root.join("tests"))? {
        let file_path = entry?.path();
        let name = file_path.file_name().map(|name| name.to_string_lossy());
        if file_path.is_file()
            && name.as_deref().is_some_and(|name| {
                name.starts_with("validator_runtime_heartbeat_")
                    && name != "validator_runtime_heartbeat_reference_registration.rs"
            })
        {
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
